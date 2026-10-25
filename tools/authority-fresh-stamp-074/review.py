"""Pure preregistered projection/installed-leaf CPU review; never admission."""
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location('fd_identity_review', Path(__file__).parents[1] / 'kernel-fresh-identity-074/review.py')
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
require = common.require


def review(cohorts, installed=False):
    require(len(cohorts) == 8, 'Not eight independent processes')
    normalized = []
    for cohort, raw in enumerate(cohorts):
        text = raw.decode('ascii')
        if installed:
            require(text.count('test result: ok. 1 passed; 0 failed; 0 ignored;') == 1, 'Missing installed verdict')
            rows = []
            for line in text.splitlines():
                marker = 'HC074-STAMP-INSTALLED '
                if marker in line:
                    require(line.count(marker) == 1, 'Duplicate marker')
                    rows.append(marker + line.split(marker, 1)[1])
            require(len(rows) == 36, 'Incomplete installed work')
            # The shared parser is a frozen schedule/CPU estimator, not a change of work.
            for row in rows:
                require(common.fields(row)['calls'] == '1024', 'Changed installed work')
                require(common.fields(row)['variant'] in ('reference', 'reference-control', 'fresh-stamp'), 'Wrong original variant')
            header = f'HC074-FD-IDENTITY-ORIGIN cohort={cohort} files=7 iterations=8192 samples=12 product=false admission=false'
            converted = [header] + [row.replace('HC074-STAMP-INSTALLED ', 'HC074-FD-IDENTITY ').replace('calls=1024 ', 'calls=57344 ').replace('variant=fresh-stamp ', 'variant=fstat-prototype ') for row in rows]
        else:
            lines = text.splitlines()
            require(len(lines) == 37 and lines[0].startswith('HC074-STAMP-ORIGIN '), 'Incomplete leaf')
            require(all(line.startswith('HC074-STAMP ') for line in lines[1:]), 'Unexpected leaf record')
            require(all(common.fields(line)['variant'] in ('reference', 'reference-control', 'fresh-stamp') for line in lines[1:]), 'Wrong original leaf variant')
            converted = [lines[0].replace('HC074-STAMP-ORIGIN ', 'HC074-FD-IDENTITY-ORIGIN ')] + [line.replace('HC074-STAMP ', 'HC074-FD-IDENTITY ').replace('variant=fresh-stamp ', 'variant=fstat-prototype ') for line in lines[1:]]
        normalized.append(('\n'.join(converted)+'\n').encode('ascii'))
    result = common.review(normalized)
    return dict(control=result['control'], candidate=result['candidate'],
                same_function_control_passed=result['same_function_control_passed'],
                cpu_screen_passed=result['leaf_screen_passed'],
                process_log_estimates=result['process_log_estimates'],
                scope='installed authority leaf' if installed else 'eleven-field projection',
                actual_calls_per_sample=1024 if installed else 57344,
                full_preparation_improvement_proven=False, product=False, admission=False)
