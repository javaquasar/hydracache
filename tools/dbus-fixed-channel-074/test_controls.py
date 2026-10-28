import json,re,unittest,tomllib
from pathlib import Path
import run,native
class Controls(unittest.TestCase):
 def test_all_original_sources_and_projection_are_whole_pinned(self):
  for name in run.NAMES:
   raw=(run.SOURCE/name).read_bytes();actual,_,_=run.project(name,raw)
   self.assertEqual((run.BASE/'input/assets/tools/dbus-fixed-channel-074'/name).read_bytes(),actual)
   with self.assertRaises(ValueError):run.project(name,raw+b'\n')
 def test_kernel_predicates_bodies_and_original_alarm_tests_unchanged(self):
  for name in ('diagnostic_manager_channel_kernel.rs','diagnostic_manager_channel_kernel_tests.rs','diagnostic_manager_alarm_tests.rs'):
   raw=(run.SOURCE/name).read_bytes();actual,_,_=run.project(name,raw)
   if name=='diagnostic_manager_channel_kernel.rs':raw=raw.replace(b'#[path = "diagnostic_manager_receiver_pin.rs"]\npub(super) mod receiver;\n',b'',1)
   self.assertEqual(actual,raw)
 def test_exact_two_second_origin_bound_and_sandbox_observations(self):
  channel=(run.BASE/'input/assets/tools/dbus-fixed-channel-074/diagnostic_manager_channel.rs').read_bytes()
  kernel=(run.BASE/'input/assets/tools/dbus-fixed-channel-074/diagnostic_manager_channel_kernel.rs').read_bytes()
  self.assertIn(b'checked_add(2_000_000_000)',kernel)
  self.assertEqual(channel.count(b'.max_queued(std::num::NonZeroUsize::new(16).unwrap())'),1)
  self.assertEqual(channel.count(b'.method_timeout(Duration::from_millis(250))'),1)
  self.assertIn(b'Builder::unix_stream(io(stream.into_std())?)',channel)
  self.assertEqual(channel.count(b'async fn bound<T>'),1)
 def test_host_and_original_join_boundaries_are_explicit_not_stubbed(self):
  channel=(run.BASE/'input/assets/tools/dbus-fixed-channel-074/diagnostic_manager_channel.rs').read_text(encoding='utf8')
  for item in ('mod original_join;','mod held_delivery;'):self.assertNotIn(item,channel)
  raw=(run.BASE/'prepare/manual-host-tests-not-executed.rs').read_bytes();self.assertEqual(raw.count(b'#[ignore ='),2)
  tests=(run.BASE/'input/assets/tools/dbus-fixed-channel-074/diagnostic_manager_channel_tests.rs').read_bytes();self.assertEqual(tests.count(b'#[test]'),5)
 def test_complete_95_case_catalog_and_test_only_registration(self):
  review=json.loads((run.BASE/'prepare/result.json').read_bytes());self.assertEqual(len(set(review['baseline']+review['extras'])),95)
  self.assertEqual(sum(n.startswith('fixed_channel::') for n in review['extras']),17)
  self.assertIn('#[cfg(all(test,target_os="linux"))]',(run.BASE/'input/fd/probe/src/lib.rs').read_text(encoding='utf8'))
 def test_source_dependency_and_product_custody(self):
  review=json.loads((run.BASE/'prepare/result.json').read_bytes());self.assertEqual(run.files(run.BASE/'input'),review['files'])
  self.assertEqual(run.prior.parent.parent.fragment.prior.parent.old.root_inputs(),review['root_inputs'])
  previous=run.ROOT/'target/dbus-legacy-calls-20261028/fixed-sender/input'
  for variant in ('baseline','fd'):
   before=tomllib.loads((previous/variant/'Cargo.lock').read_text(encoding='utf8'))['package']
   after=tomllib.loads((run.BASE/'input'/variant/'Cargo.lock').read_text(encoding='utf8'))['package']
   self.assertEqual(len(after),63)
   self.assertEqual([n for n in after if n['name'] not in ('tokio','tokio-macros')],[n for n in before if n['name']!='tokio'])
   old=next(n for n in before if n['name']=='tokio');new=next(n for n in after if n['name']=='tokio')
   self.assertEqual(new,dict(old,dependencies=sorted(old['dependencies']+['tokio-macros'])))
   macro=next(n for n in after if n['name']=='tokio-macros');self.assertEqual(macro['version'],'2.7.0')
   self.assertEqual(macro['checksum'],'385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496')
   self.assertEqual(macro['dependencies'],['proc-macro2','quote','syn 2.0.119'])
   for crate in ('zbus','zbus_macros','zbus_utils'):self.assertEqual(run.files(run.BASE/'input'/variant/crate),run.files(previous/variant/crate))
 def test_same_native_owner_and_fixed_msrv_command(self):
  owner=native.configured(run.ROOT,2);self.assertEqual(owner.BASE,run.BASE);self.assertEqual(owner.NATIVE.as_posix(),run.NATIVE)
  argv,env,_,_,_,_=owner.command('fd-188-clippy',{})
  for item in ('--offline','--locked','-D'):self.assertIn(item,argv)
  self.assertEqual(env['CARGO_INCREMENTAL'],'0')
 def test_fixed_dispatch(self):
  self.assertEqual(run.native_command(4)[-1],'session')
  with self.assertRaises(ValueError):run.native_command(3)
if __name__=='__main__':unittest.main()
