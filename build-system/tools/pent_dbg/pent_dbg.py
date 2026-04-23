import os
import sys
base_dir = os.path.dirname(os.path.abspath(__file__))
if base_dir not in sys.path:
    sys.path.insert(0, base_dir)

import gdb # type: ignore
import time
import getpa
import print_idt
import websrv
from events import shutdown_event

class Executor:
    def __init__(self, command):
        self.command = command
    def __call__(self):
        gdb.execute(self.command, from_tty=True)

class CommandThread(gdb.Thread):
    def run(self):
        while True:
            time.sleep(1)

# Register commands
command_getpa = getpa.GetPA()
command_print_idt = print_idt.PrintIDT()

# Threads
ws = websrv.WebServerThread()
ws.start()

def exited_handler(e):
    gdb.execute("quit")

def exit_handler(e):
    shutdown_event.set()

    import requests
    print("Making req")
    requests.post('http://127.0.0.1:5566/shutdown')

gdb.events.exited.connect(exited_handler)
gdb.events.gdb_exiting.connect(exit_handler)

print("PyGDB loaded")
