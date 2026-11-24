import Foundation
import XCTest
@testable import NativeShell

final class QueuedEditsTests: XCTestCase {
    @MainActor
    func testQueuedEditsCommitInOrderWithCapturedValues() throws {
        CrepusRustActions.install()
        let model = CrepusStateStore.shared
        let initialRevision = model.revision
        let first = "first \"quoted\" \\ newline\n雪"
        let final = "final 🦋"
        let expectedMessages = [first, "second", final, final, final, final, final]
        let installedSink = CrepusActions.resultSink
        var messages: [String] = []
        var actions: [String] = []
        CrepusActions.resultSink = { raw in
            XCTAssertTrue(Thread.isMainThread)
            installedSink(raw)
            let result = try! JSONSerialization.jsonObject(with: Data(raw.utf8)) as! [String: Any]
            XCTAssertEqual(result["ok"] as? Bool, true, raw)
            messages.append(model.text("note"))
            actions.append(result["action"] as! String)
        }
        CrepusActions.performChange(nil, bind: "note", value: first)
        CrepusActions.performChange("sync", bind: "note", value: "second")
        CrepusActions.performChange(nil, bind: "note", value: final)
        CrepusActions.perform("sync")
        CrepusActions.performChange(nil, bind: "enabled", value: true)
        CrepusActions.performChange(nil, bind: "ratio", value: 13.5)
        let payload = NSMutableDictionary(dictionary: ["text": "captured"])
        CrepusActions.performChange(nil, bind: "snapshot", value: payload)
        payload["text"] = "mutated after submission"
        // All submitted edits retain the sink captured before replacement.
        CrepusActions.resultSink = { _ in XCTFail("replacement sink used by queued edit") }
        let deadline = Date().addingTimeInterval(10)
        while messages.count < expectedMessages.count && Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.005))
        }
        XCTAssertEqual(messages, expectedMessages)
        XCTAssertEqual(actions, ["bind", "sync", "bind", "sync", "bind", "bind", "bind"])
        XCTAssertEqual(model.revision, initialRevision + 7)
        XCTAssertEqual(model.text("note"), final)
        XCTAssertTrue(model.bool("enabled"))
        XCTAssertEqual(model.number("ratio"), 13.5)
        XCTAssertEqual(model.text("snapshot.text"), "captured")

        var rejected = false
        CrepusActions.resultSink = { raw in
            let result = try! JSONSerialization.jsonObject(with: Data(raw.utf8)) as! [String: Any]
            XCTAssertEqual(result["ok"] as? Bool, false)
            rejected = true
        }
        CrepusActions.performChange(nil, bind: "ratio", value: Double.nan)
        XCTAssertTrue(rejected)
        XCTAssertEqual(model.number("ratio"), 13.5)
        print("PASS fresh Swift host: FIFO edits/ordinary actions, captured sink/value, final text/bool/number, invalid JSON rejection")
    }
}
