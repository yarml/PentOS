import gdb # type: ignore
from flask import Flask, jsonify, request

from events import shutdown_event
import logging


app = Flask('pygdb')

log = logging.getLogger("werkzeug")
log.setLevel(logging.ERROR)
log.propagate = False

@app.route('/read_mem')
def route_read_mem():
    return jsonify('ReadMem')

@app.route("/shutdown", methods=["POST"])
def route_shutdown():
    print("Got req")
    shutdown_func = request.environ.get("werkzeug.server.shutdown")
    if shutdown_func is None:
        print("was a dream after all")
        return "Not running with Werkzeug", 500
    shutdown_event.set()
    shutdown_func()
    print("huh?")
    return "Shutting down", 200

class WebServerThread(gdb.Thread):
    def run(self):
        pass
        # app.run(port=5566, debug=False, threaded=True)
