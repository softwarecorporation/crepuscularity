import SwiftUI

struct MotionView: View {
    @StateObject private var host = MotionHost()
    @Environment(\.accessibilityReduceMotion) private var reducedMotion
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Shared motion").font(.largeTitle.bold())
            Text("SwiftUI · Rust session engine").foregroundStyle(.secondary)
            Button(action: host.toggle) {
                Label(host.expanded ? "Hide build targets" : "Show build targets",
                      systemImage: host.expanded ? "chevron.up" : "chevron.down")
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 8)
            }
            .accessibilityIdentifier("disclosure")
            VStack(alignment: .leading, spacing: 8) {
                Text(host.text).font(.headline)
                Text("Reverse while moving. Native reduced motion settles immediately.")
                    .foregroundStyle(.secondary)
            }
            .padding(16)
            .frame(maxWidth: .infinity, minHeight: 112, maxHeight: 112, alignment: .topLeading)
            .background(.quaternary)
            .frame(height: host.height, alignment: .top)
            .clipped()
            .accessibilityHidden(!host.expanded)
            Text("System reduced motion: \(reducedMotion ? "on" : "off")")
                .accessibilityIdentifier("motion-policy")
            Text("Frames: \(host.frames) · \(host.active ? "Active" : "Idle") · Height: \(host.height, specifier: "%.1f")")
                .font(.system(.caption, design: .monospaced))
                .accessibilityIdentifier("motion-status")
            if let error = host.error { Text(error).foregroundStyle(.red) }
            Spacer(minLength: 0)
        }
        .padding(28)
        .frame(maxWidth: 560, alignment: .leading)
        .onAppear { host.setReducedMotion(reducedMotion) }
        .onChange(of: reducedMotion) { _, value in host.setReducedMotion(value) }
        .onChange(of: scenePhase) { _, value in
            if value != .active { host.suspend() }
        }
        .onDisappear { host.suspend() }
    }
}

@main
struct MotionApp: App {
    var body: some Scene {
        WindowGroup("Crepuscularity Motion") { MotionView() }
    }
}
