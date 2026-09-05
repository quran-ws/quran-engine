import SwiftUI
import UIKit
import QvpKit

@main
struct DemoApp: App {
    var body: some Scene { WindowGroup { ContentView() } }
}

/// The QvpPageView is owned by the model so panel actions can relayout / redraw it.
struct PageViewRep: UIViewRepresentable {
    let view: QvpPageView
    func makeUIView(context: Context) -> QvpPageView { view }
    func updateUIView(_ uiView: QvpPageView, context: Context) {}
}

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    func makeUIViewController(context: Context) -> UIActivityViewController { UIActivityViewController(activityItems: items, applicationActivities: nil) }
    func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}

struct ContentView: View {
    @StateObject private var m = DemoModel()

    var body: some View {
        VStack(spacing: 0) {
            topBar
            PageViewRep(view: m.view).frame(maxWidth: .infinity, maxHeight: .infinity)
            panel.frame(height: 330)
        }
        .background(m.bgColor.ignoresSafeArea())
        .preferredColorScheme(m.theme == "dark" ? .dark : .light)
        .overlay(alignment: .top) {
            if let t = m.toast {
                Text(t).font(.footnote).padding(10).background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 8)).padding(.top, 56).transition(.opacity)
            }
        }
        .sheet(item: $m.share) { ShareSheet(items: [$0.url]) }
    }

    private var topBar: some View {
        HStack(spacing: 6) {
            TextField("2:255 · Yasin · juz 30", text: $m.gotoField).textFieldStyle(.roundedBorder).font(.footnote).submitLabel(.go).onSubmit { m.goto() }
            Button("◀") { m.stepPage(-1) }.buttonStyle(.bordered)
            TextField("1", text: $m.pageField).textFieldStyle(.roundedBorder).font(.footnote).keyboardType(.numberPad).multilineTextAlignment(.center).frame(width: 56).onSubmit { m.gotoPageField() }
            Button("▶") { m.stepPage(+1) }.buttonStyle(.bordered)
        }
        .padding(.horizontal, 8).padding(.vertical, 4)
    }

    private var panel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 4) {
                section("Search this page")
                TextField("الله · الرحمان", text: $m.searchField).textFieldStyle(.roundedBorder).multilineTextAlignment(.trailing).environment(\.layoutDirection, .rightToLeft)
                ForEach(m.results, id: \.word) { r in
                    Button { m.selectWord(r.word) } label: {
                        HStack { Spacer(); Text("\(r.text)  \(r.wid)\(r.loose ? " ~" : "")").font(.system(size: 15)) }
                    }.buttonStyle(.plain).padding(.vertical, 1)
                }
                if m.searchedEmpty { Text("no match on this page").font(.caption).opacity(0.6) }

                section("Selection")
                HStack { Spacer(); Text(m.selWord).font(.system(size: 26)).multilineTextAlignment(.trailing) }
                Text(m.selInfo).font(.caption)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 4) {
                        ForEach(m.chips) { c in
                            Button(c.label) { m.toggleChip(c.id) }.font(.caption2).buttonStyle(.bordered).tint(c.on ? .orange : .gray)
                        }
                    }
                }
                HStack {
                    Button("Copy + citation") { m.copySelection() }.buttonStyle(.bordered)
                    Button("Crop → SVG") { m.cropSelection() }.buttonStyle(.bordered)
                }
                Text("Tap a word · long-press and drag to select · tap an ayah marker · chips recolour one path (e.g. 2nd diacritic)").font(.caption2).opacity(0.7)

                section("Highlights (engine-animated)")
                HStack {
                    Picker("mode", selection: $m.hlModeIdx) { Text("band + ink").tag(0); Text("band").tag(1); Text("ink").tag(2) }.pickerStyle(.segmented)
                    Toggle("Follow words", isOn: $m.playing).toggleStyle(.button).font(.caption)
                }
                slider("fade ms", $m.hlMs, 0...800)

                section("Styling (each toggle is one engine handle)")
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack {
                        Toggle("Mark colours", isOn: $m.markColours).toggleStyle(.button)
                        Toggle("Hide marks", isOn: $m.hideMarks).toggleStyle(.button)
                        Toggle("Gold markers", isOn: $m.goldMarkers).toggleStyle(.button)
                    }.font(.caption)
                }
                HStack {
                    Text("Theme").font(.caption)
                    Picker("Theme", selection: $m.theme) { Text("Light").tag("light"); Text("Sepia").tag("sepia"); Text("Dark").tag("dark") }.pickerStyle(.segmented)
                    Button("Clear all") { m.clearAll() }.buttonStyle(.bordered).font(.caption)
                }

                section("Memorisation")
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack {
                        Button("Mask ayah") { m.maskAyah() }.buttonStyle(.bordered)
                        Picker("mask", selection: $m.maskModeIdx) { Text("hide").tag(0); Text("block").tag(1) }.pickerStyle(.segmented).frame(width: 120)
                        Button("Reveal") { m.revealNext() }.buttonStyle(.bordered)
                        Button("Hide back") { m.hideBack() }.buttonStyle(.bordered)
                        Button("Unmask") { m.unmask() }.buttonStyle(.bordered)
                    }.font(.caption)
                }
                HStack {
                    Toggle("Greyed page", isOn: $m.revealOn).toggleStyle(.button).font(.caption)
                    Slider(value: $m.revealPos, in: 0...m.revealMax, step: 1).disabled(!m.revealOn)
                    Text(m.revealVal).font(.caption2).frame(minWidth: 44, alignment: .trailing)
                }

                section("Layout (engine)")
                slider("line spacing ×100", $m.lineSpacing, 60...220)
                slider("pad top", $m.padTop, 0...120)
                slider("pad bottom", $m.padBottom, 0...120)
                HStack {
                    Toggle("Fill screen height", isOn: $m.fillHeight).toggleStyle(.button)
                    Button("Leading to fill") { m.leadingToFill() }.buttonStyle(.bordered)
                }.font(.caption)

                section("Page")
                Text(m.meta).font(.caption2)
                section("Engine")
                Text(m.hudText).font(.system(size: 10.5, design: .monospaced))
            }
            .padding(.horizontal, 12).padding(.bottom, 8)
        }
    }

    private func section(_ t: String) -> some View {
        Text(t.uppercased()).font(.system(size: 10.5, weight: .bold)).opacity(0.7).padding(.top, 10)
    }
    private func slider(_ label: String, _ v: Binding<Double>, _ r: ClosedRange<Double>) -> some View {
        HStack {
            Text(label).font(.caption).frame(width: 110, alignment: .leading)
            Slider(value: v, in: r, step: 1)
            Text("\(Int(v.wrappedValue))").font(.caption).frame(width: 34, alignment: .trailing)
        }
    }
}
