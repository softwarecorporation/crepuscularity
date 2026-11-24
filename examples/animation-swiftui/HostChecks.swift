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
        waitUntil { host!.frames > count && (host!.height > 0 || !host!.active) }
        if host!.active {
            precondition(host!.height > 0 && host!.height < 112)
            let intermediate = host!.lastSample!
            host!.toggle()
            let reversed = host!.lastSample!
            // Use the timestamps of the rendered samples, not time spent waiting
            // for callbacks. Both curves have slope <= 2; allow Float IR rounding.
            let elapsedMs = Double(reversed.timeMs - intermediate.timeMs)
            precondition(abs(reversed.height - intermediate.height) <= 112 * 2 * elapsedMs / 2_000 + 0.0001)
            if host!.active {
                precondition(reversed.height > 0 && reversed.height <= 112)
                count = host!.frames
                waitUntil { host!.frames > count && (host!.height < reversed.height || !host!.active) }
                precondition(host!.height >= 0 && host!.height < reversed.height)
                if reversed.height == 112 {
                    print("Opening completed before retarget sample; PASS observed closing frame")
                } else {
                    print("PASS display-link intermediate and reversed frames")
                }
            } else {
                precondition(reversed.height == 0)
                print("Retarget sample completed closing after a delay; deterministic ABI checks cover midflight reversal")
            }
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
