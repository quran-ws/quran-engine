import XCTest
@testable import QvpKit

final class QvpPageCacheTests: XCTestCase {
    @MainActor private func waitUntil(_ condition: @MainActor () -> Bool) async throws {
        for _ in 0..<200 {
            if condition() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("Page cache did not reach the expected state")
    }

    @MainActor func testLoadsBeforeCurrentPageAndEvictsAtCapacity() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let bytes = try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
        let cache = QvpPageCache(capacity: 4, data: { _ in bytes })
        for number in 1...5 {
            _ = cache.controller(for: number)
            try await waitUntil { cache.page(for: number) != nil }
        }
        XCTAssertNil(cache.page(for: 1))
        XCTAssertEqual((1...5).filter { cache.page(for: $0) != nil }.count, 4)
    }

    @MainActor func testPermanentControllerReattachesAndProtectsNeighbors() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let bytes = try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
        var configured = 0
        let cache = QvpPageCache(capacity: 4, data: { _ in bytes }, configure: { _, _ in configured += 1 })
        cache.setCurrentPage(42)
        let controller = cache.controller(for: 42)
        try await waitUntil { (41...43).allSatisfy { cache.page(for: $0) != nil } }
        let original = try XCTUnwrap(controller.page)
        for number in 100...104 {
            _ = cache.controller(for: number)
            try await waitUntil { cache.page(for: number) != nil }
        }
        XCTAssertTrue((41...43).allSatisfy { cache.page(for: $0)?.isOpen == true })
        cache.setCurrentPage(100)
        try await waitUntil { controller.page == nil }
        XCTAssertFalse(original.isOpen)
        cache.setCurrentPage(42)
        try await waitUntil { controller.page != nil }
        XCTAssertTrue(cache.controller(for: 42) === controller)
        XCTAssertFalse(controller.page === original)
        XCTAssertTrue(controller.page?.isOpen == true)
        XCTAssertGreaterThan(configured, 4)
    }

    @MainActor func testFailedLoadCanRetry() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let bytes = try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
        var attempts = 0
        let cache = QvpPageCache(data: { _ in
            attempts += 1
            return attempts == 1 ? nil : bytes
        })
        let controller = cache.controller(for: 42)
        try await waitUntil { attempts == 1 }
        XCTAssertNil(controller.page)
        _ = cache.controller(for: 42)
        try await waitUntil { controller.page != nil }
        XCTAssertEqual(attempts, 2)
    }

    @MainActor func testRetainedControllerKeepsPageOpenAfterCacheRelease() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let bytes = try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
        var cache: QvpPageCache? = QvpPageCache(data: { _ in bytes })
        let controller = try XCTUnwrap(cache).controller(for: 42)
        try await waitUntil { controller.page != nil }
        weak var releasedCache = cache
        cache = nil
        XCTAssertNil(releasedCache)
        XCTAssertTrue(controller.page?.isOpen == true)
        controller.setBounds(CGSize(width: 345, height: 550), fromCanvas: 1)
        XCTAssertNotNil(controller.page?.currentLayout)
    }

    @MainActor func testControllerOperationsIgnoreClosedPage() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let page = try QvpPage(bytes: Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp")))
        let controller = QvpCanvasController()
        controller.page = page
        controller.setBounds(CGSize(width: 345, height: 550), fromCanvas: 1)
        XCTAssertTrue(page.isOpen)
        page.close()
        page.close()
        XCTAssertFalse(page.isOpen)
        controller.relayout()
        controller.clearSelection()
        controller.selectTo(.zero)
        XCTAssertNil(controller.hitAt(.zero))
    }

    @MainActor func testPreloadRespectsPageRange() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { return }
        let bytes = try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
        var requests: [Int] = []
        let cache = QvpPageCache(pageRange: 1...2, data: { number in
            requests.append(number)
            return bytes
        })
        cache.setCurrentPage(1)
        try await waitUntil { cache.page(for: 1) != nil && cache.page(for: 2) != nil }
        cache.setCurrentPage(2)
        XCTAssertEqual(requests.sorted(), [1, 2])
    }
}
