#!/usr/bin/env python3
"""Package already-built native executables. Release tooling only."""
import argparse
import gzip
import io
from pathlib import Path
import subprocess
import tarfile
import zipfile


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target',required=True)
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[1]
    windows='windows' in args.target
    suffix='.exe' if windows else ''
    files={name+suffix:root/'target'/args.target/'release'/(name+suffix) for name in ('kurage','kurage-export')}
    for name in ('README.md','LICENSE','THIRD_PARTY_NOTICES.md'):
        files[name]=root/name
    for name in ('DejaVu','FreeType'):
        files[f'licenses/{name}.txt']=root/f'assets/render/LICENSE-{name}.txt'
    out=root/'output/packages'
    out.mkdir(parents=True,exist_ok=True)
    stem='kurage-aquarium-'+args.target
    if windows:
        destination=out/(stem+'.zip')
        with zipfile.ZipFile(destination,'w',compression=zipfile.ZIP_DEFLATED) as archive:
            for name,path in files.items(): archive.writestr(name,path.read_bytes())
    else:
        destination=out/(stem+'.tar.gz')
        mtime=int(subprocess.check_output(['git','show','-s','--format=%ct','HEAD'],cwd=root,text=True))
        with destination.open('wb') as output:
            with gzip.GzipFile(filename='',mode='wb',fileobj=output,mtime=mtime) as compressed:
                with tarfile.open(fileobj=compressed,mode='w') as archive:
                    for name,path in files.items():
                        data=path.read_bytes()
                        item=tarfile.TarInfo(name)
                        item.size=len(data)
                        item.mode=0o755 if name in ('kurage','kurage-export') else 0o644
                        item.mtime=mtime
                        archive.addfile(item,io.BytesIO(data))
    print(destination)


if __name__=='__main__':main()
