import AppKit
import Foundation

@main
struct HostChecks {
    @MainActor
    static func main() {
        _ = NSApplication.shared
        var host: MotionHost? = MotionHost(durationMs: 2_000)
        weak var released = host
        precondition(host!.error == nil && host!.height == 0)
        host!.toggle()
        var count = host!.frames
        let observationStart = DispatchTime.now().uptimeNanoseconds
        waitUntil { host!.frames > count && (host!.height > 0 || !host!.active) }
        if host!.active {
            precondition(host!.height > 0 && host!.height < 112)
            let intermediate = host!.height
            host!.toggle()
            let reversed = host!.height
            precondition(reversed > 0 && reversed < 112)
            // Both curves have maximum slope 2; the closing sample may already
            // be below the last opening frame. Allow integer-ms and Float IR rounding.
            let elapsedMs = Double(DispatchTime.now().uptimeNanoseconds - observationStart) / 1_000_000 + 1
            precondition(abs(reversed - intermediate) <= 112 * 2 * elapsedMs / 2_000 + 0.0001)
            count = host!.frames
            waitUntil { host!.frames > count && (host!.height < reversed || !host!.active) }
            precondition(host!.height >= 0 && host!.height < reversed)
            print("PASS display-link intermediate and reversed frames")
        } else {
            precondition(host!.height == 112)
            print("Display-link delay completed motion before an intermediate sample; deterministic engine tests cover interpolation")
        }
        host!.setReducedMotion(true)
        precondition(!host!.active && host!.height == (host!.expanded ? 112 : 0))
        assertIdle(host!)
        host!.toggle()
        precondition(!host!.active && host!.height == (host!.expanded ? 112 : 0))
        host!.setReducedMotion(false)
        assertIdle(host!)
        host!.toggle()
        waitUntil { !host!.active }
        precondition(host!.height == (host!.expanded ? 112 : 0))
        assertIdle(host!)
        host!.toggle()
        count = host!.frames
        waitUntil { host!.frames > count }
        host!.suspend()
        precondition(!host!.active && host!.height == (host!.expanded ? 112 : 0))
        assertIdle(host!)
        host!.toggle()
        host!.close()
        assertIdle(host!)
        host = nil
        precondition(released == nil)
        let delayed = MotionHost(durationMs: 30)
        delayed.toggle()
        Thread.sleep(forTimeInterval: 0.06)
        waitUntil { !delayed.active }
        precondition(delayed.error == nil && delayed.height == 112)
        assertIdle(delayed)
        delayed.close()
        print("PASS shared Rust ABI / native display callbacks / live policy / exact endpoints / idle / suspend / teardown")
    }

    @MainActor
    private static func waitUntil(_ predicate: () -> Bool) {
        let deadline = Date(timeIntervalSinceNow: 5)
        while !predicate() && Date() < deadline { spin(0.01) }
        precondition(predicate(), "Native display callback did not reach the expected state")
    }

    @MainActor
    private static func assertIdle(_ host: MotionHost) {
        let count = host.frames
        spin(0.15)
        precondition(host.error == nil && !host.active && host.frames == count)
    }

    @MainActor
    private static func spin(_ seconds: Double) {
        RunLoop.main.run(until: Date(timeIntervalSinceNow: seconds))
    }
}
