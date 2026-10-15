import argparse
import json
import os
from pathlib import Path
import resource
import subprocess
import time


def capture(command):
    return subprocess.check_output(command, text=True)


def inventory(selectors):
    data = json.loads(capture([
        'cargo', 'nextest', 'list', *selectors, '--message-format', 'json',
    ]))
    result = {}
    for binary, suite in data['rust-suites'].items():
        def listed(*options):
            output = capture([suite['binary-path'], '--list', *options])
            return {line[:-6] for line in output.splitlines() if line.endswith(': test')}

        cases = suite['testcases']
        if listed() != set(cases):
            raise RuntimeError(f'{binary}: Cargo and nextest test names differ')
        ignored = {name for name, case in cases.items() if case['ignored']}
        if listed('--ignored') != ignored:
            raise RuntimeError(f'{binary}: ignored test lists differ')
        if any(case['filter-match'] != (
            {'status': 'mismatch', 'reason': 'ignored'} if case['ignored'] else {'status': 'matches'}
        ) for case in cases.values()):
            raise RuntimeError(f'{binary}: unexpected nextest default filter')
        result[binary] = {
            'tests': sorted(set(cases) - ignored), 'ignored': sorted(ignored),
        }
    if not result or not any(item['tests'] for item in result.values()):
        raise RuntimeError('No ordinary tests selected')
    return result


def main():
    parser = argparse.ArgumentParser(description='Compare equivalent warm CI test selections in Docker/Linux.')
    parser.add_argument('--suite', choices=['standard', 'large'], required=True)
    parser.add_argument('--threads', type=int, nargs='+', default=[2, 8])
    parser.add_argument('--repeats', type=int, default=2)
    parser.add_argument('--nextest-bin', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.repeats < 2 or any(threads < 1 for threads in args.threads):
        parser.error('Use at least two repeats and positive thread counts')
    os.environ['PATH'] = str(args.nextest_bin.resolve()) + os.pathsep + os.environ['PATH']
    os.environ['CARGO_TERM_COLOR'] = 'never'
    args.output.mkdir(parents=True, exist_ok=False)
    metadata = json.loads(capture(['cargo', 'metadata', '--locked', '--all-features', '--no-deps', '--format-version', '1']))
    selector = (
        "const {suiteCommands}=require('./.github/scripts/test-suites.cjs');"
        "let input='';process.stdin.on('data',chunk=>input+=chunk);"
        "process.stdin.on('end',()=>process.stdout.write(JSON.stringify("
        "suiteCommands(JSON.parse(input),process.argv[1]))));"
    )
    commands = json.loads(subprocess.check_output(
        ['node', '-e', selector, args.suite], input=json.dumps(metadata), text=True,
    ))
    if len(commands) != 1:
        raise RuntimeError('This comparison requires one default workspace package')
    selectors = commands[0][1:]
    subprocess.run(['cargo', 'test', *selectors, '--no-run'], check=True)
    tests = inventory(selectors)
    report = {
        'revision': capture(['git', 'rev-parse', 'HEAD']).strip(),
        'working_tree': capture(['git', 'status', '--short']),
        'rustc': capture(['rustc', '--version', '--verbose']),
        'nextest': capture(['cargo', 'nextest', '--version']),
        'logical_cpus': os.cpu_count(),
        'suite': args.suite, 'inventory': tests, 'runs': [],
    }
    destination = args.output / 'results.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    for repeat in range(1, args.repeats + 1):
        settings = args.threads if repeat % 2 else list(reversed(args.threads))
        for threads in settings:
            runners = ['cargo', 'nextest'] if repeat % 2 else ['nextest', 'cargo']
            for runner in runners:
                if runner == 'cargo':
                    command = ['cargo', 'test', *selectors, '--', f'--test-threads={threads}']
                else:
                    command = [
                        'cargo', 'nextest', 'run', *selectors, '--test-threads', str(threads),
                        '--no-fail-fast', '--status-level', 'fail', '--final-status-level', 'fail',
                    ]
                log = args.output / f'{runner}-{threads}-{repeat}.log'
                before = resource.getrusage(resource.RUSAGE_CHILDREN)
                started = time.monotonic()
                with log.open('w') as stream:
                    process = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT)
                wall = time.monotonic() - started
                after = resource.getrusage(resource.RUSAGE_CHILDREN)
                cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
                row = {
                    'runner': runner, 'threads': threads, 'repeat': repeat,
                    'wall_seconds': round(wall, 3), 'cpu_seconds': round(cpu, 3),
                    'average_cpu_cores': round(cpu / wall, 3),
                    'returncode': process.returncode, 'command': command, 'log': log.name,
                }
                report['runs'].append(row)
                destination.write_text(json.dumps(report, indent=2) + '\n')
                print(json.dumps(row), flush=True)
                if process.returncode:
                    raise RuntimeError(f'Test run failed; inspect {log}')


if __name__ == '__main__':
    main()
