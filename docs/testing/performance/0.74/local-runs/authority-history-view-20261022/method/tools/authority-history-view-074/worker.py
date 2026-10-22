"""Fresh offline historical reader process. Never executes Rust/model images."""
import importlib
import json
from pathlib import Path
import sys

sys.path.insert(0,str(Path(__file__).resolve().parent))
import view_inputs as view


def replay(root,m):
    root=Path(root).resolve();before=view.verify_tree(root,m)
    sys.path.insert(0,str(root/'tools/offline-authority-integration-audit-074'))
    old=importlib.import_module('audit_integration')
    for name in m['reader_modules']:
        expected=(root/name).resolve()
        loaded=[mod for mod in tuple(sys.modules.values()) if getattr(mod,'__file__',None) and Path(mod.__file__).resolve()==expected]
        view.need(len(loaded)==1,'Historical reader module not loaded from exact view')
    view.need(old.ROOT==root,'Historical reader ROOT escaped the view')
    with view.read_only_scope(root):
        actual=old.analyze(root);expected=old.load(root/m['expected_facts'])
        old.exact(actual,expected,'Historical facts differ')
    after=view.verify_tree(root,m);view.need(before==after,'Historical tree changed during replay')
    return dict(schema='authority-history-worker-074-v1',history_source=view.SOURCE,history_only=True,
        historical_facts_equal=True,old_readers_unchanged=True,read_only_path_scope=True,
        expected_facts_sha256=view.sha((root/m['expected_facts']).read_bytes()),tree=after,
        targeted_model_gate=actual['targeted_model_gate'],prior_broad_isolation_gate=actual['prior_broad_isolation_gate'],
        original_windows_seconds=actual['original_windows_seconds'],first_refusals=actual['first_refusals'],
        qualification_sha256=actual['qualification_sha256'],runtime_candidate_implemented=False,
        runtime_candidate_guard_implemented=False,model_images_executed=0,original_owner_profiles=0,
        clocks_changed=False,services_changed=False,server=False,product=False)


if __name__=='__main__':
    view.need(len(sys.argv)==2,'One generated historical root required')
    print(json.dumps(replay(sys.argv[1],view.validate_manifest(view.decode(sys.stdin.read()))),sort_keys=True))
