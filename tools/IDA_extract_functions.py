import idc
from idautils import *
from idaapi import *
import json

filename = 'idaoutput.txt'
results = {'symbols': []}

for f in Functions(0):
    results['symbols'].append(
        {'name': get_func_name(f), 'start': f - get_imagebase(), 'end': idc.find_func_end(f) - get_imagebase() }
    )

print(json.dumps(results, indent=4))
with open(filename, 'w+', encoding='utf-8') as f:
    f.write(json.dumps(results))

print(f'JSON dumped to: {filename}')