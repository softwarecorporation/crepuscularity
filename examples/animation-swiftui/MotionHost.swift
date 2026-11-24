import Foundation
import QuartzCore
import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif

// Decode only the layout and text this host displays; the Rust session owns interpolation.
private struct Document: Decodable {
    struct Node: Decodable {
        struct Style: Decodable { let height: Double? }
        let style: Style?
        let children: [Node]?
        let content: String?
    }
    let root: [Node]
}

@MainActor
final class MotionHost: NSObject, ObservableObject {
    @Published private(set) var height = 0.0
    @Published private(set) var text = ""
    @Published private(set) var expanded = false
    @Published private(set) var active = false
    @Published private(set) var frames = 0
    @Published private(set) var error: String?
    private var session: OpaquePointer?
    private var displayLink: CADisplayLink?
    private var reducedMotion = false

    override init() {
        super.init()
        session = crepus_session_new()
        do {
            guard session != nil else { throw failure() }
            try check(crepus_session_set_template_string(session,
                "div #panel class=\"h-[{height}px]\"\n  span \"One engine, native frames.\"", nil))
            try check(crepus_session_set_context_json(session, "{\"height\":0}"))
            try render()
        } catch { fail(error) }
    }

    deinit {
        displayLink?.invalidate()
        if let session { crepus_session_free(session) }
    }

    func toggle() {
        guard session != nil, error == nil else { return }
        expanded.toggle()
        do {
            let request = """
            {"key":"height","target":\(expanded ? 112 : 0),"durationMs":240,
             "easing":"ease-in-out","nowMs":\(Self.now),"reducedMotion":\(reducedMotion)}
            """
            try check(crepus_session_animate_json(session, request))
            try sample()
            schedule()
        } catch { fail(error) }
    }

    func setReducedMotion(_ value: Bool) {
        reducedMotion = value
        guard session != nil, error == nil else { return }
        do {
            // Also render policy changes when idle; switching policy off must not replay motion.
            try sample()
            schedule()
        } catch { fail(error) }
    }

    func suspend() {
        stopFrames()
        guard session != nil, error == nil else { return }
        do {
            try check(crepus_session_tick_animations(session, Self.now, 1))
            active = false
            try render()
        } catch { fail(error) }
    }

    func close() {
        stopFrames()
        if let session { crepus_session_free(session) }
        session = nil
        active = false
    }

    private static var now: UInt64 { DispatchTime.now().uptimeNanoseconds / 1_000_000 }

    @objc private func frame(_ link: CADisplayLink) {
        do {
            try sample()
            if !active { stopFrames() }
        } catch { fail(error) }
    }

    private func sample() throws {
        let result = crepus_session_tick_animations(session, Self.now, reducedMotion ? 1 : 0)
        try check(result)
        active = result == 1
        try render() // The final inactive sample contains the exact endpoint.
    }

    private func render() throws {
        guard let raw = crepus_session_render_ir_json(session) else { throw failure() }
        defer { crepus_string_free(raw) }
        let document = try JSONDecoder().decode(Document.self, from: Data(String(cString: raw).utf8))
        guard let panel = document.root.first, let height = panel.style?.height,
              let content = panel.children?.first?.content else {
            throw NSError(domain: "MotionHost", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Missing panel layout in View IR"])
        }
        self.height = height
        text = content
        frames += 1
    }

    private func schedule() {
        guard active else { stopFrames(); return }
        guard displayLink == nil else { return }
        #if os(macOS)
        guard let screen = NSScreen.main else { return }
        let link = screen.displayLink(target: self, selector: #selector(frame(_:)))
        #else
        let link = CADisplayLink(target: self, selector: #selector(frame(_:)))
        #endif
        displayLink = link
        link.add(to: .main, forMode: .common)
    }

    private func stopFrames() {
        displayLink?.invalidate()
        displayLink = nil
    }

    private func check(_ result: Int32) throws {
        if result < 0 { throw failure() }
    }

    private func failure() -> Error {
        guard let raw = crepus_session_take_last_error(session) else {
            return NSError(domain: "MotionHost", code: 1)
        }
        defer { crepus_string_free(raw) }
        return NSError(domain: "MotionHost", code: 1,
                       userInfo: [NSLocalizedDescriptionKey: String(cString: raw)])
    }

    private func fail(_ error: Error) {
        stopFrames()
        active = false
        self.error = error.localizedDescription
    }
}
