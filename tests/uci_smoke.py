#!/usr/bin/env python3
"""Exercise an actual native UCI process, including mid-search control."""
import queue
import subprocess
import sys
import threading
import time


class Session:
    def __init__(self, executable):
        self.process = subprocess.Popen(
            [executable], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, bufsize=1,
        )
        self.lines = queue.Queue()
        threading.Thread(target=self.collect, daemon=True).start()

    def collect(self):
        for line in self.process.stdout:
            self.lines.put(line.strip())
        self.lines.put(None)

    def send(self, command):
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()

    def until(self, prefix, timeout=10):
        deadline = time.monotonic() + timeout
        seen = []
        while True:
            line = self.lines.get(timeout=max(0.01, deadline - time.monotonic()))
            if line is None:
                raise AssertionError("engine exited before " + prefix)
            seen.append(line)
            if line.startswith(prefix):
                return seen
            if time.monotonic() >= deadline:
                raise AssertionError("engine timeout: " + repr(seen[-10:]))

    def close(self):
        if self.process.poll() is None:
            self.send("quit")
            self.process.wait(timeout=5)
        assert self.process.returncode == 0, self.process.stderr.read()


def main(executable):
    engine = Session(executable)
    try:
        engine.send("uci")
        identification = engine.until("uciok")
        assert any("uncalibrated" in line for line in identification)
        assert any("UCI_Chess960" in line for line in identification)
        assert any("Skill_Level type spin default 21 min 1 max 21" in line for line in identification)
        engine.send("isready")
        engine.until("readyok")
        engine.send("setoption name SyzygyPath value /definitely/not/a/tablebase")
        engine.until("info string rejected:")
        engine.send("setoption name SyzygyPath value")
        engine.until("info string Syzygy tablebases disabled")
        engine.send("setoption name SyzygyPath value /definitely/not/a/tablebase")
        engine.until("info string rejected:")
        engine.send("setoption name SyzygyPath value")
        disabled = engine.until("info string Syzygy tablebases disabled")
        assert any("Syzygy tablebases disabled" in line for line in disabled)

        engine.send("position startpos")
        engine.send("go depth 3")
        normal = engine.until("bestmove ")
        assert any("info depth 3 " in line for line in normal), normal
        assert normal[-1] != "bestmove 0000"

        engine.send("position startpos")
        engine.send("go infinite")
        engine.send("isready")
        engine.until("readyok", timeout=5)
        engine.send("stop")
        stopped = engine.until("bestmove ", timeout=5)
        assert stopped[-1] != "bestmove 0000"

        engine.send("position startpos")
        engine.send("go ponder wtime 2000 btime 2000")
        pondering = engine.until("info depth ")
        assert not any(line.startswith("bestmove ") for line in pondering)
        engine.send("ponderhit")
        pondered = engine.until("bestmove ", timeout=5)
        assert pondered[-1] != "bestmove 0000", pondered

        engine.send("position startpos")
        engine.send("go depth 2 searchmoves e2e4")
        assert engine.until("bestmove ")[-1] == "bestmove e2e4"

        engine.send("position fen 7k/8/6KQ/8/8/8/8/8 w - - 149 1")
        engine.send("go depth 2")
        mating = engine.until("bestmove ")
        assert any("score mate 1" in line for line in mating), mating

        engine.send("position fen 7k/6Q1/6K1/8/8/8/8/8 b - - 150 1")
        engine.send("go depth 4")
        assert engine.until("bestmove ")[-1] == "bestmove 0000"

        engine.send("setoption name Mode value human-like")
        engine.send("setoption name UCI_Elo value 500")
        engine.send("setoption name UCI_LimitStrength value true")
        engine.send("position startpos")
        engine.send("go depth 64")
        weak = engine.until("bestmove ")
        assert weak[-1] != "bestmove 0000"
        assert not any("multipv 2" in line for line in weak), weak
        engine.send("setoption name UCI_Elo value 499")
        engine.until("info string rejected:")
        engine.send("isready")
        engine.until("readyok")

        engine.send("setoption name UCI_LimitStrength value false")
        engine.send("setoption name UCI_Chess960 value true")
        engine.send("position fen 4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1")
        engine.send("go depth 1 searchmoves f1g1")
        assert engine.until("bestmove ")[-1] == "bestmove f1g1"
        engine.send("setoption name Ponder value true")
        engine.send("position startpos")
        engine.send("go depth 3")
        hint = engine.until("bestmove ")[-1]
        assert " ponder " in hint, hint
        print("Native UCI smoke: handshake, search, stop, ready, ponder, roots, mate, presets, Chess960 passed")
    finally:
        engine.close()


if __name__ == "__main__":
    main(sys.argv[1])
