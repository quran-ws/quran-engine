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
// `reconfigure()` after a theme change to restyle every live page.
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

    @ObservationIgnored private let capacity: Int
    @ObservationIgnored private let pageRange: ClosedRange<Int>
    @ObservationIgnored private let data: (Int) async -> Data?
    @ObservationIgnored private let configure: (QvpPage, QvpCanvasController) -> Void

    /// - Parameters:
    ///   - capacity: live `QvpPage`s kept at most (the current page and its
    ///     immediate neighbors are never evicted, whatever their age).
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

    /// Tell the cache where the pager is: the page and its neighbors are
    /// (re)loaded — which also restores a previously evicted page onto a
    /// still-living view — and become ineligible for eviction.
    public func setCurrentPage(_ pageNumber: Int) {
        currentPage = pageNumber
        for neighbor in [pageNumber - 1, pageNumber, pageNumber + 1] where pageRange.contains(neighbor) {
            ensureLoaded(neighbor)
        }
        trim()
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

    /// The current page and its immediate neighbors are never evicted.
    private func isProtected(_ pageNumber: Int) -> Bool {
        guard let current = currentPage else { return false }
        return pageNumber >= current - 1 && pageNumber <= current + 1
    }

    /// Close least-recently-used pages beyond capacity. Never the current
    /// page or its immediate neighbors.
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

    deinit {
        for page in pages.values { page.close() }
    }
}
#endif
