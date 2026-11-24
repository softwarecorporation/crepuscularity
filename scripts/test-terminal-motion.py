#!/usr/bin/env python3
"""Exercise the actual terminal host through a private tmux PTY, not mocked events."""
import os
from pathlib import Path
import re
import shlex
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
SOCKET = f"crepus-motion-{os.getpid()}"


def tmux(*args):
    return subprocess.check_output(["tmux", "-L", SOCKET, *args], text=True)


def screen():
    return tmux("capture-pane", "-p", "-t", "motion")


def state():
    text = screen()
    match = re.search(r"Frames: (\d+) \| (Active|Idle) \| Reduced motion: (true|false) \| Height: ([\d.]+)", text)
    if not match:
        return None
    return int(match[1]), match[2], match[3] == "true", float(match[4])


def until(predicate):
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        current = state()
        if current and predicate(current):
            return current
        time.sleep(0.005)
    raise AssertionError(f"Terminal did not reach expected state:\n{screen()}")


def idle():
    before = until(lambda s: s[1] == "Idle")
    time.sleep(0.3)
    assert state() == before, "Frames continued while idle"
    return before


def key(value):
    tmux("send-keys", "-t", "motion", value)


def start(binary, reduced=False):
    command = f"exec {shlex.quote(str(binary))}" + (" --reduce-motion" if reduced else "")
    tmux("-f", "/dev/null", "new-session", "-d", "-s", "motion", "-x", "100", "-y", "24", command)
    tmux("set-option", "-g", "exit-empty", "off")
    return until(lambda s: s[1] == "Idle")


def main():
    subprocess.run(["cargo", "build", "--locked", "-p", "crepuscularity-tui", "--example", "animated_disclosure"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug/examples/animated_disclosure"
    try:
        assert start(binary)[3] == 0
        idle()
        assert "One engine, terminal frames" not in screen()
        key("Enter")
        opening = until(lambda s: s[1] == "Active" and 0 < s[3] < 8)
        key("Enter")
        reversed_frame = until(lambda s: s[1] == "Active" and 0 < s[3] < opening[3])
        assert idle()[3] == 0
        assert "One engine, terminal frames" not in screen()
        key("Enter")
        until(lambda s: s[1] == "Active" and s[3] > 0)
        key("r")
        assert until(lambda s: s[1] == "Idle" and s[2])[3] == 8
        idle()
        key("Enter")
        assert until(lambda s: s[1] == "Idle" and s[3] == 0)[2]
        key("r")
        assert not until(lambda s: not s[2])[2]
        key("Space")
        assert until(lambda s: s[1] == "Idle" and s[3] == 8)[1] == "Idle"
        idle()
        key("Enter")
        until(lambda s: s[1] == "Active")
        # Unhandled input must not keep resetting the frame deadline.
        deadline = time.monotonic() + 0.5
        while time.monotonic() < deadline:
            key("x")
            time.sleep(0.005)
        assert state()[1] == "Idle" and state()[3] == 0
        assert idle()[3] == 0
        tmux("kill-session", "-t", "motion")
        assert start(binary, reduced=True)[2]
        key("Enter")
        assert until(lambda s: s[3] == 8)[1] == "Idle"
        idle()
        tmux("resize-window", "-t", "motion", "-x", "80", "-y", "20")
        assert idle()[3] == 8
        key("q")
        deadline = time.monotonic() + 3
        while subprocess.run(["tmux", "-L", SOCKET, "has-session", "-t", "motion"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            assert time.monotonic() < deadline, "Q did not exit"
            time.sleep(0.01)
        print(f"PASS actual PTY: intermediate={opening[3]:.3f}, reversed={reversed_frame[3]:.3f}; live/startup reduced motion, immediate Space, idle frame counts, input flood and exit")
    finally:
        subprocess.run(["tmux", "-L", SOCKET, "kill-server"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
