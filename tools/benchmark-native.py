#!/usr/bin/env python3
"""Compare native export variants, including full GIF equality and CPU time."""
import argparse
import hashlib
import json
from pathlib import Path
import resource
import statistics
import subprocess
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',action='append',required=True,help='label=/path/to/executable')
    parser.add_argument('--runs',type=int,default=3)
    parser.add_argument('--workers',type=int,default=4)
    parser.add_argument('--baseline-ref',help='source version/commit for the first executable')
    parser.add_argument('--rss',action='store_true',help='sample generator peak RSS from Linux /proc (encoder excluded)')
    parser.add_argument('--output',type=Path,default=Path('output/buffer-pipeline/benchmark.json'))
    args=parser.parse_args()
    if args.rss and not Path('/proc/self/status').exists():parser.error('--rss requires Linux /proc')
    binaries=[(label,Path(path).resolve()) for label,path in (v.split('=',1) for v in args.binary)]
    assert args.runs>0 and args.workers>0
    results=[]
    with tempfile.TemporaryDirectory(prefix='kurage-native-bench-') as temp:
        temp=Path(temp)
        for single in (False,True):
            scene='single' if single else 'aquarium'
            for run in range(args.runs):
                order=binaries[run%len(binaries):]+binaries[:run%len(binaries)]
                for label,binary in order:
                    output=temp/'scene.gif'
                    command=[str(binary),str(output),'--workers',str(args.workers)]+(['--single'] if single else [])
                    usage=resource.getrusage(resource.RUSAGE_CHILDREN)
                    before_cpu=usage.ru_utime+usage.ru_stime
                    start=time.perf_counter()
                    peak_rss=0
                    with (temp/'render.log').open('w') as log:
                        if args.rss:
                            with subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT) as process:
                                while process.poll() is None:
                                    try:
                                        status=Path(f'/proc/{process.pid}/status').read_text()
                                        value=next((int(line.split()[1]) for line in status.splitlines() if line.startswith('VmHWM:')),0)
                                        peak_rss=max(peak_rss,value)
                                    except (FileNotFoundError,ProcessLookupError):pass
                                    time.sleep(.02)
                                returncode=process.returncode
                        else:returncode=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT).returncode
                    if returncode:
                        raise RuntimeError((temp/'render.log').read_text())
                    elapsed=time.perf_counter()-start
                    usage=resource.getrusage(resource.RUSAGE_CHILDREN)
                    cpu=usage.ru_utime+usage.ru_stime-before_cpu
                    digest=hashlib.sha256(output.read_bytes()).hexdigest()
                    results.append({'implementation':label,'scene':scene,'run':run+1,'seconds':elapsed,
                        'cpu_seconds':cpu,'bytes':output.stat().st_size,'sha256':digest})
                    if args.rss:results[-1]['generator_peak_rss_kib']=peak_rss
                    output.unlink();output.with_suffix('.png').unlink()
                    print(f'{scene} {label} run {run+1}: {elapsed:.2f}s wall, {cpu:.2f}s CPU',flush=True)
                    if args.rss:print(f'  generator peak RSS: {peak_rss} KiB (FFmpeg excluded)',flush=True)
    for scene in ('aquarium','single'):
        matching=[r for r in results if r['scene']==scene]
        assert len({r['sha256'] for r in matching})==1,f'{scene} GIF mismatch'
        for label,_ in binaries:
            values=[r['seconds'] for r in matching if r['implementation']==label]
            print(scene,label,'median',round(statistics.median(values),3),flush=True)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'baseline':args.baseline_ref or binaries[0][0],'workers':args.workers,
        'executables':{label:hashlib.sha256(binary.read_bytes()).hexdigest() for label,binary in binaries},
        'runs':results},indent=2)+'\n')


if __name__=='__main__':main()
