
## Input JSON structure

```json
{
  "functions": [
    {
      "name": "sub_140001000",
      "start": 4096,
      "end": 4230
    },
    [... more entries ...]
  ]
}
```

## IDA script


### IDA python

```python
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
```

### IDC (IDA free)
```
extern g_idcutil_logfile;
static LogInit()
{
  g_idcutil_logfile = fopen("idaout.txt", "w+");
  if (g_idcutil_logfile == 0)
    return 0;
  return 1;
}

static main()
{
    LogInit();
    fprintf(g_idcutil_logfile, "%s", "{\"functions\":[");
    msg("%s", "{\"functions\":[");
    auto ea, x;  for ( ea=NextFunction(0); ea != BADADDR; ea=NextFunction(ea) )
    {
        fprintf(g_idcutil_logfile, "{\"name\": \"%s\" ,  \"start\" : %ld, \"end\": %ld}\n", GetFunctionName(ea), ea - get_imagebase(), find_func_end(ea) - get_imagebase());
        msg("{\"name\": \"%s\" ,  \"start\" : %ld, \"end\": %ld}", GetFunctionName(ea), ea - get_imagebase(), find_func_end(ea) - get_imagebase());
        if (NextFunction(ea) != BADADDR) {
            msg("%s", ",\n");
            fprintf(g_idcutil_logfile, "%s", ",\n");
        }
    }

    fprintf(g_idcutil_logfile, "%s", "]}");
    msg("%s", "]}\n");
    
    msg("Saved to idaout.txt");
    fclose(g_idcutil_logfile);
}
```

This should output a valid JSON file named "idaout.txt" next to the file your oppened in IDA.