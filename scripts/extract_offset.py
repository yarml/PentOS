import sys,json

pts = json.load(sys.stdin)['partitiontable']
ss = pts['sectorsize']

[print(p['node'],p['start'] * ss // (1024 * 1024)) for p in pts['partitions']]
