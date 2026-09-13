// A simple Quran reader over QvpKit, using stock iOS components: NavigationStack + toolbars,
// sheets with Form / List / .searchable, a Menu for memorisation. Tap a word or an ayah mark to highlight it.
import SwiftUI
import UIKit
import QvpKit

@main
struct DemoApp: App {
    var body: some Scene { WindowGroup { ReaderView() } }
}

/// The QvpPageView is owned by the model so actions can relayout / redraw it.
struct PageViewRep: UIViewRepresentable {
    let view: QvpPageView
    func makeUIView(context: Context) -> QvpPageView { view }
    func updateUIView(_ uiView: QvpPageView, context: Context) {}
}

// MARK: - Reader

struct ReaderView: View {
    @StateObject private var m = DemoModel()
    @State private var showGoTo = false
    @State private var showSearch = false
    @State private var showSettings = false

    var body: some View {
        reader.onAppear {
            switch UserDefaults.standard.string(forKey: "qvpSheet") {   // screenshots / QA
            case "settings": showSettings = true
            case "search": showSearch = true
            case "goto": showGoTo = true
            default: break
            }
        }
    }

    /// Fill-screen mode is immersive: no bars, the page owns the whole screen, a floating button brings the controls back.
    private var immersive: Bool { m.fillHeight && !m.controlsShown }

    private var reader: some View {
        NavigationStack {
            ZStack(alignment: .bottom) {
                PageViewRep(view: m.view).padding(.bottom, immersive ? 0 : 58).ignoresSafeArea(.keyboard)
                    .overlay(alignment: .top) {
                        if let img = m.flipImage {
                            Image(uiImage: img).resizable().aspectRatio(contentMode: .fit)
                                .offset(x: m.flipOffset).shadow(radius: 8).allowsHitTesting(false)
                        }
                    }
                if m.revealOn { revealBar }
            }
            .background(m.bgColor.ignoresSafeArea())
            .overlay(alignment: .topTrailing) { if immersive { showControlsButton } }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar(immersive ? .hidden : .visible, for: .navigationBar, .bottomBar)
            .statusBarHidden(immersive)
            .animation(.easeInOut(duration: 0.2), value: immersive)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    VStack(spacing: 0) {
                        Text(m.title).font(.headline)
                        Text(m.subtitle).font(.caption).foregroundStyle(.secondary)
                    }
                    .accessibilityIdentifier("pageTitle")
                }
                ToolbarItemGroup(placement: .topBarLeading) {
                    Button { showGoTo = true } label: { Label("Go to", systemImage: "list.bullet") }
                    if m.fillHeight {
                        Button { withAnimation { m.controlsShown = false } } label: { Label("Hide controls", systemImage: "arrow.up.left.and.arrow.down.right") }
                    }
                }
                ToolbarItemGroup(placement: .topBarTrailing) {
                    Button { showSearch = true } label: { Label("Search", systemImage: "magnifyingglass") }
                    Button { showSettings = true } label: { Label("Settings", systemImage: "textformat.size") }
                }
                ToolbarItemGroup(placement: .bottomBar) {
                    Button { m.flip(-1) } label: { Label("Previous page", systemImage: "chevron.left") }
                    Spacer()
                    Button { m.playing.toggle() } label: { Label(m.playing ? "Stop following" : "Follow words", systemImage: m.playing ? "pause.fill" : "play.fill") }
                    Spacer()
                    memoriseMenu
                    Spacer()
                    Button { m.flip(+1) } label: { Label("Next page", systemImage: "chevron.right") }
                }
            }
        }
        .tint(m.theme == "sepia" ? .brown : .accentColor)
        .preferredColorScheme(m.theme == "dark" ? .dark : .light)
        .overlay(alignment: .top) {
            if let t = m.toast {
                Text(t).font(.footnote).lineLimit(3).padding(10)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
                    .padding(.top, 8).padding(.horizontal).transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeInOut, value: m.toast)
        .sheet(isPresented: $showGoTo) { GoToSheet(m: m) }
        .sheet(isPresented: $showSearch) { SearchSheet(m: m) }
        .sheet(isPresented: $showSettings) { SettingsSheet(m: m) }
    }

    /// Floating, translucent, in the top-right safe area: the only chrome left while the page fills the screen.
    private var showControlsButton: some View {
        Button { withAnimation { m.controlsShown = true } } label: {
            Image(systemName: "chevron.down")
                .font(.body.weight(.semibold))
                .frame(width: 40, height: 40)
                .background(.regularMaterial, in: Circle())
                .shadow(color: .black.opacity(0.15), radius: 4, y: 1)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Show controls")
        .padding(.top, 8).padding(.trailing, 12)
        .transition(.opacity)
    }

    private var memoriseMenu: some View {
        Menu {
            Section("Mask the current ayah") {
                Button { m.maskModeIdx = 0; m.maskAyah() } label: { Label("Hide words", systemImage: "eye.slash") }
                Button { m.maskModeIdx = 1; m.maskAyah() } label: { Label("Cover words", systemImage: "rectangle.fill") }
            }
            Button { m.unmaskNext() } label: { Label("Reveal next word", systemImage: "arrow.right.circle") }
            Button { m.maskBack() } label: { Label("Hide last revealed", systemImage: "arrow.left.circle") }
            Button { m.unmask() } label: { Label("Show everything", systemImage: "eye") }
            Divider()
            Toggle(isOn: $m.revealOn) { Label("Greyed page", systemImage: "circle.lefthalf.filled") }
        } label: { Label("Memorise", systemImage: m.revealOn ? "brain.head.profile.fill" : "brain.head.profile") }
    }

    private var revealBar: some View {
        HStack {
            Image(systemName: "circle.lefthalf.filled").foregroundStyle(.secondary)
            Slider(value: $m.revealPos, in: 0...max(m.revealMax, 1), step: 1).disabled(m.revealMax < 1)
            Text(m.revealVal).font(.caption.monospacedDigit()).foregroundStyle(.secondary).frame(minWidth: 44, alignment: .trailing)
        }
        .padding(.horizontal).padding(.vertical, 10)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
        .padding(.horizontal).padding(.bottom, 8)
    }
}

// MARK: - Go to

struct GoToSheet: View {
    @ObservedObject var m: DemoModel
    @Environment(\.dismiss) private var dismiss
    @State private var ayahKey = ""
    @State private var query = ""
    private var surahs: [QvpAtlasSurah] { m.atlas?.surahs() ?? [] }
    private var filtered: [QvpAtlasSurah] {
        query.isEmpty ? surahs : (m.atlas?.searchSurahs(query) ?? [])
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    HStack {
                        TextField("Ayah, e.g. 2:255", text: $ayahKey).keyboardType(.numbersAndPunctuation).submitLabel(.go).onSubmit(goAyah)
                        Button("Go", action: goAyah).disabled(ayahKey.isEmpty)
                    }
                } footer: {
                    Text("The whole mushaf is bundled — 604 pages. Type an ayah key, pick a juz, or search a surah by name or number.")
                }
                Section("Juz") {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            ForEach(1...30, id: \.self) { j in
                                Button("\(j)") { if let d = m.atlas?.juz(j) { m.loadPage(d.page); dismiss() } }
                                    .buttonStyle(.bordered).controlSize(.small)
                            }
                        }
                    }
                }
                Section("Surahs") {
                    ForEach(filtered, id: \.n) { s in
                        Button { m.loadPage(s.page); dismiss() } label: {
                            HStack {
                                Text("\(s.n)").font(.caption.monospacedDigit()).foregroundStyle(.secondary).frame(width: 30, alignment: .trailing)
                                VStack(alignment: .leading) {
                                    Text(s.latin).font(.body)
                                    Text("\(s.english) · \(s.ayahCount) ayahs · page \(s.page)").font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Text(s.arabic).font(.title3)
                            }
                        }
                        .tint(.primary)
                    }
                }
            }
            .searchable(text: $query, prompt: "Surah name or number")
            .navigationTitle("Go to")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
        .presentationDetents([.medium, .large])
    }
    private func goAyah() { m.gotoField = ayahKey; m.goto(); dismiss() }
}

// MARK: - Search

struct SearchSheet: View {
    @ObservedObject var m: DemoModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Group {
                if m.searchField.isEmpty {
                    ContentUnavailableView("Search this page", systemImage: "magnifyingglass", description: Text("Type a word; the engine normalises tashkil and hamzah forms (الرحمان finds الرحمن)."))
                } else if m.searchedEmpty {
                    ContentUnavailableView.search(text: m.searchField)
                } else {
                    List(m.results, id: \.word) { r in
                        Button { m.selectWord(r.word); dismiss() } label: {
                            HStack {
                                Text(r.wordKey).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                                if r.isLooseMatch { Text("≈").foregroundStyle(.secondary) }
                                Spacer()
                                Text(r.text).font(.title3)
                            }
                        }.tint(.primary)
                    }
                }
            }
            .searchable(text: $m.searchField, placement: .navigationBarDrawer(displayMode: .always), prompt: "الله · الرحمان")
            .navigationTitle("Search")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
        .presentationDetents([.medium, .large])
    }
}

// MARK: - Settings

struct SettingsSheet: View {
    @ObservedObject var m: DemoModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section("Appearance") {
                    Picker("Theme", selection: $m.theme) { Text("Light").tag("light"); Text("Sepia").tag("sepia"); Text("Dark").tag("dark") }.pickerStyle(.segmented)
                    Toggle("Coloured marks", isOn: $m.markColours)
                    Toggle("Hide tashkil", isOn: $m.hideMarks)
                    Toggle("Gold ayah marks", isOn: $m.goldAyahMarks)
                }
                Section("Layout") {
                    Toggle("Fill screen height", isOn: $m.fillHeight)
                    LabeledContent("Line spacing") { Text(String(format: "×%.2f", m.lineSpacing / 100)).monospacedDigit() }
                    Slider(value: $m.lineSpacing, in: 100...220, step: 5) { Text("Line spacing") }
                    LabeledContent("Top padding") { Text("\(Int(m.padTop)) pt").monospacedDigit() }
                    Slider(value: $m.padTop, in: 0...120, step: 4) { Text("Top padding") }
                    LabeledContent("Bottom padding") { Text("\(Int(m.padBottom)) pt").monospacedDigit() }
                    Slider(value: $m.padBottom, in: 0...120, step: 4) { Text("Bottom padding") }
                    Button("Add leading to fill the screen") { m.leadingToFill() }
                    Text("Leading only grows — the printed lineSpacing is the floor, so the lines never close up — and the text width is always the screen's.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Section("Highlights") {
                    Picker("Style", selection: $m.hlModeIdx) { Text("Band + ink").tag(0); Text("Band").tag(1); Text("Ink").tag(2) }
                    LabeledContent("Fade") { Text("\(Int(m.hlMs)) ms").monospacedDigit() }
                    Slider(value: $m.hlMs, in: 0...800, step: 50) { Text("Fade") }
                }
                Section("This page") { Text(m.meta).font(.caption).foregroundStyle(.secondary) }
                Section("Engine") {
                    Text(m.hudText).font(.caption.monospaced()).foregroundStyle(.secondary).accessibilityIdentifier("engineStats")
                }
                Section { Button("Reset styles, highlights and masks", role: .destructive) { m.clearAll() } }
            }
            .navigationTitle("Reading")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
    }
}
