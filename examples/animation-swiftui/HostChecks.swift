import AppKit
import Foundation

@main
struct HostChecks {
    @MainActor
    static func main() {
        _ = NSApplication.shared
        var host: MotionHost? = MotionHost()
        weak var released = host
        precondition(host!.error == nil && host!.height == 0)
        host!.toggle()
        spin(0.075)
        precondition(host!.active && host!.height > 0 && host!.height < 112)
        let intermediate = host!.height
        host!.toggle()
        precondition(host!.height >= intermediate && host!.height < 112)
        spin(0.06)
        precondition(host!.height > 0 && host!.height < intermediate)
        host!.setReducedMotion(true)
        precondition(!host!.active && host!.height == 0)
        assertIdle(host!)
        host!.toggle()
        precondition(!host!.active && host!.height == 112)
        host!.setReducedMotion(false)
        assertIdle(host!)
        host!.toggle()
        spin(0.35)
        precondition(!host!.active && host!.height == 0)
        assertIdle(host!)
        host!.toggle()
        spin(0.04)
        host!.suspend()
        precondition(!host!.active && host!.height == 112)
        assertIdle(host!)
        host!.toggle()
        host!.close()
        assertIdle(host!)
        host = nil
        precondition(released == nil)
        print("PASS shared Rust ABI / display-link intermediate and reversed frames / live policy / exact endpoints / idle / suspend / teardown")
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
