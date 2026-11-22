// @quran.ws/qvp-react-native, iOS. The bytes a URI names, and the word records of a page.
import Foundation

/// The bytes a prop-given URI names. The schemes are Android's: on iOS an asset is a file in the
/// app bundle, so `asset://pages/001.qvp` is `pages/001.qvp` under the bundle's resources.
enum QvpRnBytes {
    static func read(_ uri: String) throws -> Data {
        let u = uri.trimmingCharacters(in: .whitespacesAndNewlines)
        if u.hasPrefix("base64:") {
            guard let d = Data(base64Encoded: String(u.dropFirst(7)), options: .ignoreUnknownCharacters) else { throw QvpRnError("not base64: \(u.prefix(32))") }
            return d
        }
        return try Data(contentsOf: URL(fileURLWithPath: path(u)))
    }
    private static func path(_ u: String) -> String {
        if u.hasPrefix("file://") { return URL(string: u)?.path ?? String(u.dropFirst(7)) }
        if u.hasPrefix("/") { return u }
        let asset = u.hasPrefix("asset://") ? String(u.dropFirst(8)) : u.hasPrefix("asset:") ? String(u.dropFirst(6)) : u
        return (Bundle.main.resourcePath ?? "") + "/" + asset
    }
}

/// An error with a message JavaScript reads.
struct QvpRnError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

/// The bridge record of each word of one page, built the first time the word is asked for and
/// kept. A record holds the word's text forms, its label and the names of its paths. The words
/// sidecar is attached once per page, so a record does not change until the next attach, and
/// the view drops this table then. The module's query queue and the main thread both call it.
final class QvpRnWords {
    let page: QvpPage
    private let lock = NSLock()
    private var cachedForms: [Form]?
    private var records: [[String: Any?]?]

    init(_ page: QvpPage) { self.page = page; records = Array(repeating: nil, count: page.nWords) }

    /// The forms this page has. Each one is a scan of every word in the engine, so read them once.
    var forms: [Form] {
        lock.lock(); defer { lock.unlock() }
        return formsLocked()
    }
    private func formsLocked() -> [Form] {
        if let f = cachedForms { return f }
        let f = QvpRnMarshal.forms(page)
        cachedForms = f
        return f
    }

    /// The record of word `index`, or nil when the page has no such word.
    func word(_ index: Int) -> [String: Any?]? {
        guard index >= 0, index < page.words.count else { return nil }
        lock.lock(); defer { lock.unlock() }
        if let r = records[index] { return r }
        let r = QvpRnMarshal.word(page, page.words[index], formsLocked())
        records[index] = r
        return r
    }

    /// Every word's record, in page order.
    func all() -> [[String: Any?]] { (0..<page.nWords).compactMap { word($0) } }
}
