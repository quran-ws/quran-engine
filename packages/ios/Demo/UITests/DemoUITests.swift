// Gesture checks against the running demo: tap → word selection, long-press + drag → multi-word
// selection, pinch → zoom (the HUD keeps reporting). Run:
//   xcodebuild test -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:DemoUITests
import XCTest

final class DemoUITests: XCTestCase {
    private func launch(_ args: [String] = []) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = ["-qvpPage", "440"] + args
        app.launch()
        XCTAssertTrue(app.otherElements["qvpPage"].waitForExistence(timeout: 10))
        return app
    }
    /// A point on the 3rd printed line of page 440 (the first ayahs of Yasin), right of centre.
    private func wordSpot(_ app: XCUIApplication) -> XCUICoordinate {
        app.otherElements["qvpPage"].coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.42))
    }

    func testTapSelectsAWord() {
        let app = launch()
        wordSpot(app).tap()
        let info = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'wid 36:'")).firstMatch
        XCTAssertTrue(info.waitForExistence(timeout: 5), "tap did not resolve to a word of surah 36")
    }

    func testTapOnEmptyPaperClearsSelection() {
        let app = launch(["-qvpWord", "5"])
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'wid '")).firstMatch.waitForExistence(timeout: 5))
        app.otherElements["qvpPage"].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.03)).tap()   // above the first line
        XCTAssertTrue(app.staticTexts["—"].waitForExistence(timeout: 5))
    }

    func testLongPressDragSelectsWords() {
        let app = launch()
        let from = wordSpot(app)
        let to = app.otherElements["qvpPage"].coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.42))
        from.press(forDuration: 0.8, thenDragTo: to)
        let info = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'selection · '")).firstMatch
        XCTAssertTrue(info.waitForExistence(timeout: 5), "long-press drag did not produce a multi-word selection")
    }

    func testPinchZoomKeepsDrawing() {
        let app = launch()
        app.otherElements["qvpPage"].pinch(withScale: 2.0, velocity: 1.0)
        let hud = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'base layer'")).firstMatch
        XCTAssertTrue(hud.waitForExistence(timeout: 5))
        XCTAssertTrue(hud.label.contains("paths in"))
    }
}
