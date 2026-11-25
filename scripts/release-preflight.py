#!/usr/bin/env python3
# Read-only release gate: exact main commit, exact full main push CI run.
import argparse
import json
import importlib.util
from pathlib import Path
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


def accept_full_or_reuse(jobs, gate_log, validate_reuse):
    try:
        validate_jobs(jobs)
        return 'full'
    except ValueError:
        # Only the successful existing CI gate's explicit reuse branch permits
        # absent platform jobs. A merely skipped/draft job cannot authorize it.
        gate = [job for job in jobs if job['name'] == 'CI required']
        if (len(gate) != 1 or gate[0].get('conclusion') != 'success'
                or not any(step['name'] == 'Validate exact results and live PR identity'
                           and step.get('conclusion') == 'success' for step in gate[0].get('steps', []))
                or not any(line.rstrip().endswith('CI required: reuse passed') for line in gate_log.splitlines())):
            raise ValueError('Neither full CI nor the verified-main reuse gate succeeded')
        validate_reuse()
        return 'verified-main'


def ci_control():
    spec = importlib.util.spec_from_file_location('ci_control', Path(__file__).with_name('ci-control.py'))
    ci = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ci)
    return ci


def validate_reused_proof(repository, sha):
    ci = ci_control()
    plan = dict(repository=repository, commit=sha, tree=ci.git('rev-parse', sha + '^{tree}'),
                config=ci.fingerprint(), rustc=ci.command('rustc', '--version'))
    pulls = ci.pages(f'repos/{repository}/commits/{sha}/pulls')
    matching = [pr for pr in pulls if pr.get('merged_at') and pr['merge_commit_sha'] == sha
                and pr['base']['ref'] == 'main' and pr['head']['repo']
                and pr['head']['repo']['full_name'] == repository
                and pr['base']['repo']['full_name'] == repository]
    ci.require(len(matching) == 1, 'No unique same-repository merged main PR')
    pr = ci.live_pr(repository, matching[0]['number'])
    ci.require(pr['merged'] and pr['merge_commit_sha'] == sha and pr['base']['ref'] == 'main'
               and pr['head']['repo']['full_name'] == repository
               and pr['base']['repo']['full_name'] == repository, 'Merged PR identity changed')
    runs = ci.pages(f'repos/{repository}/actions/workflows/ci.yml/runs?event=pull_request&head_sha={pr["head"]["sha"]}&status=success', 'workflow_runs')
    for candidate in runs:
        try:
            run = ci.api(f'repos/{repository}/actions/runs/{candidate["id"]}')
            artifacts = ci.pages(f'repos/{repository}/actions/runs/{run["id"]}/artifacts', 'artifacts')
            expected = f'ci-full-proof-{run["id"]}-{run["run_attempt"]}'
            artifacts = [item for item in artifacts if item['name'] == expected]
            ci.require(len(artifacts) == 1, 'No unique immutable full proof')
            proof = ci.read_artifact(repository, artifacts[0])
            checked_commit = ci.api(f'repos/{repository}/git/commits/{proof["commit"]}')
            # Existing CI receipt validator binds exact tree/config/toolchain,
            # merged base/head, workflow/run/attempt and three OS + DB evidence.
            ci.validate_proof(proof, run, pr, plan, checked_commit)
            source_jobs = ci.pages(f'repos/{repository}/actions/runs/{run["id"]}/jobs?filter=all', 'jobs')
            source_jobs = ci.latest_jobs(source_jobs, run['id'], run['head_sha'], run['run_attempt'])
            validate_jobs(source_jobs)
            return
        except (KeyError, ValueError, subprocess.CalledProcessError):
            continue
    raise ValueError('Verified-main source full proof is missing, expired or mismatched')


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
    pages = api(f'repos/{args.repository}/actions/runs/{run["id"]}/jobs?filter=all&per_page=100')
    jobs = ci_control().latest_jobs([job for page in pages for job in page['jobs']],
                                    run['id'], args.sha, run['run_attempt'])
    gate_jobs = [job for job in jobs if job['name'] == 'CI required']
    gate_log = ''
    try:
        validate_jobs(jobs)
    except ValueError:
        if len(gate_jobs) == 1:
            gate_log = subprocess.check_output(['gh', 'api',
                f'repos/{args.repository}/actions/jobs/{gate_jobs[0]["id"]}/logs'], text=True)
    mode = accept_full_or_reuse(jobs, gate_log, lambda: validate_reused_proof(args.repository, args.sha))
    print(f'{mode} main push {run["id"]}, attempt {run["run_attempt"]}, accepted exact SHA {args.sha}')


if __name__ == '__main__':
    main()
