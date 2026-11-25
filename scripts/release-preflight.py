#!/usr/bin/env python3
# Read-only release gate: exact main commit, exact full main push CI run.
import argparse
import json
import subprocess


def api(path):
    result = subprocess.run(['gh', 'api', path, '--paginate', '--slurp'],
                            check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def select_run(runs, sha, repository):
    exact = [run for run in runs if run.get('head_sha') == sha
             and run.get('head_branch') == 'main' and run.get('event') == 'push'
             and run.get('path') == '.github/workflows/ci.yml'
             and run.get('head_repository', {}).get('full_name') == repository]
    if not exact:
        raise ValueError('No exact ci.yml main push run exists for the tagged SHA')
    run = max(exact, key=lambda item: (item['run_number'], item['run_attempt']))
    if run.get('status') != 'completed' or run.get('conclusion') != 'success':
        raise ValueError('The latest exact main push CI run must complete successfully')
    return run


def validate_jobs(jobs):
    required = {
        'check (ubuntu-latest)': 'Record completed platform checks',
        'check (macos-latest)': 'Record completed platform checks',
        'check (windows-latest)': 'Record completed platform checks',
        'integration': 'Record completed database checks',
        'CI required': 'Validate exact results and live PR identity',
    }
    for name, full_step in required.items():
        matching = [job for job in jobs if job['name'] == name]
        if len(matching) != 1 or matching[0].get('conclusion') != 'success':
            raise ValueError(f'Missing successful full CI job: {name}')
        if not any(step['name'] == full_step and step.get('conclusion') == 'success'
                   for step in matching[0].get('steps', [])):
            raise ValueError(f'Full CI evidence missing or skipped: {name} / {full_step}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--sha', required=True)
    parser.add_argument('--repository', required=True)
    args = parser.parse_args()
    subprocess.run(['git', 'fetch', '--no-tags', 'origin',
                    'refs/heads/main:refs/remotes/origin/main'], check=True)
    subprocess.run(['git', 'merge-base', '--is-ancestor', args.sha, 'refs/remotes/origin/main'], check=True)
    pages = api(f'repos/{args.repository}/actions/workflows/ci.yml/runs?event=push&branch=main&head_sha={args.sha}&per_page=100')
    run = select_run([run for page in pages for run in page['workflow_runs']], args.sha, args.repository)
    pages = api(f'repos/{args.repository}/actions/runs/{run["id"]}/attempts/{run["run_attempt"]}/jobs?per_page=100')
    validate_jobs([job for page in pages for job in page['jobs']])
    print(f'Full CI main push {run["id"]}, attempt {run["run_attempt"]}, accepted exact SHA {args.sha}')


if __name__ == '__main__':
    main()
