import sys
import json

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f'Usage : {sys.argv[0]} file')
        exit(1)

    with open(sys.argv[1], 'r') as f:
        content = json.loads(f.read())

    for module in content:
        for f in module['symbols']:
            print(f'set_name({f["rva"]} + get_imagebase(), "{f["name"]}", 1);')