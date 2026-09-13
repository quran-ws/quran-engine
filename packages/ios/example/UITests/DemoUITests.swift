// Gesture checks against the running demo: tap → word highlight (exposed as the page's accessibility
// value), tap on empty paper → cleared, pinch → zoom (the engine stats keep reporting), search sheet,
// fill-screen mode hides the bars behind a floating button. Run:
//   xcodebuild test -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:DemoUITests
import XCTest

final class DemoUITests: XCTestCase {
    /// Launches on page 440 with the bars visible (fill-screen mode hides them by default).
    private func launch(_ args: [String] = [], controls: Bool = true) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = ["-qvpPage", "440", "-qvpControls", controls ? "1" : "0"] + args
        app.launch()
        XCTAssertTrue(app.otherElements["qvpPage"].waitForExistence(timeout: 10))
        return app
    }

    func testFillScreenHidesBarsBehindFloatingButton() {
        let app = launch(controls: false)
        XCTAssertFalse(app.buttons["Search"].exists, "fill-screen mode should start without the navigation bar")
        XCTAssertFalse(app.buttons["Next page"].exists, "fill-screen mode should start without the bottom bar")
        let show = app.buttons["Show controls"]
        XCTAssertTrue(show.waitForExistence(timeout: 5))
        show.tap()
        XCTAssertTrue(app.buttons["Search"].waitForExistence(timeout: 5), "the floating button did not bring the bars back")
        XCTAssertTrue(app.buttons["Next page"].exists)
        app.buttons["Hide controls"].tap()
        XCTAssertTrue(show.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Search"].exists)
    }
    private func pageValue(_ app: XCUIApplication) -> String { (app.otherElements["qvpPage"].value as? String) ?? "" }
    private func wait(_ app: XCUIApplication, _ cond: @escaping (String) -> Bool, timeout: Double = 5) -> Bool {
        let t0 = Date()
        while Date().timeIntervalSince(t0) < timeout { if cond(pageValue(app)) { return true }; usleep(200_000) }
        return false
    }

    func testTapHighlightsAWord() {
        let app = launch()
        // the 3rd printed line of page 440 (the first ayahs of Yasin), right of centre
        app.otherElements["qvpPage"].coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.40)).tap()
        XCTAssertTrue(wait(app, { $0.hasPrefix("word 36:") }), "tap did not resolve to a word of surah 36: \(pageValue(app))")
    }

    func testTapOnEmptyPaperClears() {
        let app = launch(["-qvpWord", "5"])
        XCTAssertTrue(wait(app, { $0.hasPrefix("word ") }))
        app.otherElements["qvpPage"].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.04)).tap()   // above the first line
        XCTAssertTrue(wait(app, { $0.isEmpty }), "tap on empty paper did not clear the highlight: \(pageValue(app))")
    }

    func testPinchZoomKeepsDrawing() {
        let app = launch()
        app.otherElements["qvpPage"].pinch(withScale: 2.0, velocity: 1.0)
        app.buttons["Settings"].tap()
        let hud = app.staticTexts["engineStats"]
        for _ in 0..<4 where !hud.exists { app.swipeUp() }          // the Engine section is below the fold of the form
        XCTAssertTrue(hud.waitForExistence(timeout: 5))
        XCTAssertTrue(hud.label.contains("paths in"))
    }

    func testSwipeFlipsPage() {
        let app = launch()
        XCTAssertTrue(app.staticTexts["Page 440 · Juz 22"].waitForExistence(timeout: 5))
        app.otherElements["qvpPage"].swipeRight()                   // mushaf order: next page
        XCTAssertTrue(app.staticTexts["Page 441 · Juz 22"].waitForExistence(timeout: 5), "swipe right did not flip to page 441")
        app.otherElements["qvpPage"].swipeLeft()
        XCTAssertTrue(app.staticTexts["Page 440 · Juz 22"].waitForExistence(timeout: 5), "swipe left did not flip back")
    }

    func testSearchSheetFindsWords() {
        let app = launch()
        app.buttons["Search"].tap()
        let field = app.searchFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.tap(); field.typeText("الله")
        XCTAssertTrue(app.staticTexts["35:45:3"].waitForExistence(timeout: 5))
        app.staticTexts["35:45:3"].tap()
        XCTAssertTrue(wait(app, { $0.hasPrefix("word 35:45:3") }), "picking a search hit did not highlight it: \(pageValue(app))")
    }
}
