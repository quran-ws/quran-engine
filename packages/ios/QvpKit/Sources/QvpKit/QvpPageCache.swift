// Page lifecycle for hosts that show many pages (a pager over the whole
// mushaf): permanent controllers, cycling page data.
//
// The invariant that makes blank pages impossible to express: a view takes
// its page's controller ONCE and keeps it for life — the cache never hands a
// page number two different controllers. Only the heavy `QvpPage` data moves:
// it is loaded on demand, evicted least-recently-used under memory pressure,
// and REATTACHED to the same permanent controller when the page comes back
// into reach. `QvpPageCanvas` observes its controller, so a reattach redraws
// with no view participation.
//
// The host supplies two closures: `data` reads a page's `.qvp` bytes (the
// cache never guesses at bundles), and `configure` applies host styling to a
// freshly attached page (ink, crop, layout mode). Call `setCurrentPage` from
// the pager so neighbors preload and stay off the eviction list; call
// `reconfigure()` after a theme change to recolorStyle every live page.
#if canImport(SwiftUI)
import SwiftUI

@available(iOS 17.0, macOS 14.0, *)
@MainActor @Observable
public final class QvpPageCache {

    @ObservationIgnored private var controllers: [Int: QvpCanvasController] = [:]
    @ObservationIgnored private var pages: [Int: QvpPage] = [:]
    @ObservationIgnored private var loads: [Int: Task<Void, Never>] = [:]
    /// LRU order of page numbers, freshest last.
    @ObservationIgnored private var recency: [Int] = []
    /// Where the pager is, once the host has said; nil until then, so nothing is
    /// protected from eviction and no distance is ever computed against a sentinel.
    @ObservationIgnored private var currentPage: Int? = nil
    /// How many pages the host shows at once, from the same call.
    @ObservationIgnored private var span: Int = 1

    @ObservationIgnored private let capacity: Int
    @ObservationIgnored private let pageRange: ClosedRange<Int>
    @ObservationIgnored private let data: (Int) async -> Data?
    @ObservationIgnored private let configure: (QvpPage, QvpCanvasController) -> Void

    /// - Parameters:
    ///   - capacity: live `QvpPage`s kept at most (the pages on screen and the
    ///     screens either side are never evicted, whatever their age — three
    ///     pages at one per screen, six at two).
    ///   - pageRange: valid page numbers (`setCurrentPage` preloads inside it).
    ///   - data: reads one page's `.qvp` bytes; called off the main actor's
    ///     critical path, may do file IO.
    ///   - configure: host styling applied every time a page is (re)attached.
    public init(capacity: Int = 12,
                pageRange: ClosedRange<Int> = 1...604,
                data: @escaping (Int) async -> Data?,
                configure: @escaping (QvpPage, QvpCanvasController) -> Void = { _, _ in }) {
        self.capacity = max(capacity, 4)
        self.pageRange = pageRange
        self.data = data
        self.configure = configure
    }

    /// The PERMANENT controller for a page — same instance for the life of
    /// the cache. Ensures the page data is loaded and attached, now or as
    /// soon as the load lands.
    public func controller(for pageNumber: Int) -> QvpCanvasController {
        let controller: QvpCanvasController
        if let existing = controllers[pageNumber] {
            controller = existing
        } else {
            controller = QvpCanvasController()
            controllers[pageNumber] = controller
        }
        ensureLoaded(pageNumber)
        return controller
    }

    /// The live page data for a page number, if currently attached.
    public func page(for pageNumber: Int) -> QvpPage? { pages[pageNumber] }

    /// Tell the cache where the pager is: the pages on screen and the screens
    /// either side are (re)loaded — which also restores a previously evicted
    /// page onto a still-living view — and become ineligible for eviction.
    ///
    /// `span` is how many pages the host shows at once and `pageNumber` the
    /// first of them, so a host laying two pages side by side passes the
    /// spread's first page and 2: the screen before it, the screen, and the
    /// screen after it are all kept ready, and a swipe never lands on a blank
    /// half. At the default 1 this is the page and its two neighbors, as it
    /// always was.
    public func setCurrentPage(_ pageNumber: Int, span: Int = 1) {
        currentPage = pageNumber
        self.span = max(span, 1)
        for neighbor in protectedRange(around: pageNumber) where pageRange.contains(neighbor) {
            ensureLoaded(neighbor)
        }
        trim()
    }

    /// The pages kept ready around `page`: one screen before it, its own
    /// screen, and one screen after — `page − span` through
    /// `page + 2·span − 1`. Takes the page rather than reading `currentPage`,
    /// so there is no "no current page yet" case to stand for: a `ClosedRange`
    /// has no empty value, and one written to mean empty traps where it is
    /// built, not where it is used.
    private func protectedRange(around page: Int) -> ClosedRange<Int> {
        (page - span)...(page + 2 * span - 1)
    }

    /// Re-run the host's `configure` on every live page and redraw — call
    /// after a theme change.
    public func reconfigure() {
        for (pageNumber, page) in pages {
            guard let controller = controllers[pageNumber] else { continue }
            configure(page, controller)
            controller.invalidate()
        }
    }

    /// Load a page's engine data and attach it to the permanent controller,
    /// unless it is already live or already loading.
    private func ensureLoaded(_ pageNumber: Int) {
        touch(pageNumber)
        guard pages[pageNumber] == nil, loads[pageNumber] == nil else { return }
        loads[pageNumber] = Task { [weak self] in
            // Single exit: the pending-load slot is ALWAYS cleared, so one
            // failed load can never poison its page number forever.
            await self?.performLoad(pageNumber)
            self?.loads[pageNumber] = nil
        }
    }

    private func performLoad(_ pageNumber: Int) async {
        guard let bytes = await data(pageNumber), let page = try? QvpPage(bytes: bytes) else { return }
        // Raced by a concurrent load that landed first: keep the winner.
        guard pages[pageNumber] == nil else { page.close(); return }
        _ = page.buildPaths()
        let controller = controller(for: pageNumber)
        configure(page, controller)
        pages[pageNumber] = page
        // Attaching redraws any live canvas showing this controller.
        controller.page = page
        touch(pageNumber)
        trim()
    }

    /// Most-recently-used order: last element = freshest.
    private func touch(_ pageNumber: Int) {
        recency.removeAll { $0 == pageNumber }
        recency.append(pageNumber)
    }

    /// The pages on screen and the screens either side are never evicted.
    /// Nothing is protected until the host has said where the pager is.
    private func isProtected(_ pageNumber: Int) -> Bool {
        guard let current = currentPage else { return false }
        return protectedRange(around: current).contains(pageNumber)
    }

    /// Close least-recently-used pages beyond capacity. Never a page on
    /// screen or on the screens either side of it.
    private func trim() {
        while pages.count > capacity {
            guard let victim = recency.first(where: { !isProtected($0) && pages[$0] != nil }),
                  let evicted = pages.removeValue(forKey: victim) else { return }
            // DETACH before closing: the permanent controller stays with any
            // view that holds it; its canvas draws nothing until the page is
            // reattached by a later ensureLoaded.
            controllers[victim]?.page = nil
            evicted.close()
            recency.removeAll { $0 == victim }
        }
    }

    // No deinit close: `QvpPage` frees its native data when its last reference goes,
    // so a controller retained by a surviving view keeps its page open after the
    // cache itself is released.
}
#endif
