#!/usr/bin/env python3
"""Compare full exports with the pinned Python release on the same machine."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import statistics
import subprocess
import tarfile
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--python',type=Path,required=True)
    parser.add_argument('--rust',type=Path,default=Path('target/release/kurage-export'))
    parser.add_argument('--runs',type=int,default=3)
    parser.add_argument('--output',type=Path,default=Path('output/migration/benchmark.json'))
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[1]
    python=args.python.resolve();rust=args.rust.resolve()
    results=[]
    data=subprocess.check_output(['git','archive','c5d795a','src'],cwd=root)
    with tempfile.TemporaryDirectory(prefix='kurage-benchmark-') as temp:
        temp=Path(temp)
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:archive.extractall(temp,filter='data')
        env=os.environ.copy();env['PYTHONPATH']=str(temp/'src')
        for single in (False,True):
            scene='single' if single else 'aquarium'
            for run in range(args.runs):
                # Alternate the implementation order to reduce order bias.
                for impl in (('python','rust') if run%2==0 else ('rust','python')):
                    destination=temp/f'{impl}-{scene}.gif'
                    command=([str(python),'-m','kurage_aquarium.export'] if impl=='python' else [str(rust)])
                    command += [str(destination),'--workers','4']+(['--single'] if single else [])
                    usage=resource.getrusage(resource.RUSAGE_CHILDREN)
                    before_cpu=usage.ru_utime+usage.ru_stime
                    start=time.perf_counter()
                    with (temp/'render.log').open('w') as log:
                        subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
                    elapsed=time.perf_counter()-start
                    usage=resource.getrusage(resource.RUSAGE_CHILDREN)
                    cpu=usage.ru_utime+usage.ru_stime-before_cpu
                    digest=hashlib.sha256(destination.read_bytes()).hexdigest()
                    results.append({'implementation':impl,'scene':scene,'run':run+1,'seconds':elapsed,
                                    'cpu_seconds':cpu,'bytes':destination.stat().st_size,'sha256':digest})
                    destination.unlink();destination.with_suffix('.png').unlink()
                    print(f'{scene} {impl} run {run+1}: {elapsed:.2f}s wall, {cpu:.2f}s CPU',flush=True)
    for scene in ('aquarium','single'):
        matching=[r for r in results if r['scene']==scene]
        assert len({r['sha256'] for r in matching})==1,f'{scene} output mismatch'
        for impl in ('python','rust'):
            subset=[r for r in matching if r['implementation']==impl]
            print(scene,impl,'median',round(statistics.median(r['seconds'] for r in subset),2),flush=True)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'source':'c5d795a','workers':4,'runs':results},indent=2)+'\n')


if __name__=='__main__':main()
