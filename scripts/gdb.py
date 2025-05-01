import gdb # type: ignore
import time

class Executor:
    def __init__(self, command):
        self.command = command
    def __call__(self):
        gdb.execute(self.command, from_tty=True)

class CommandThread(gdb.Thread):
    def run(self):
        while True:
            time.sleep(1)

# CommandThread().start()

print("Python script loaded")
