## Usage

You first need to provide a list of functions from your target. Scripts to extract them from IDA to the correct format are available under `tools/IDA_extract_functions`.
This list of functions should be a JSON file that has this shape :

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

Then, pass it as an argument along with your target and an output file.

```
cerberust.exe -i functions_list.json no_symbols_target.exe resolved_symbols.json
```

The output file will be a JSON file containing resolved symbols aswell as their PA and RVA. A script can be found under `tools/output_to_idc.py` to generate an IDA IDC script that will rename all resolved symbols to your analysis.
