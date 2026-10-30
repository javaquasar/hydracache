# Final offline verification of declined F

Verification index SHA256:
`ba395cf60db35db3de930de8db8732ab9361fcc9b8600023d1163add4019e9bc`.
Primary experiment index SHA256:
`e554bd8ee5bd7b4f0cc2963393ccbea922e93663f294962446d1caa651034403`.

All seven final offline checks pass:13 mutation/source/scope controls, Python
syntax, Rust formatting, complete byte verification, exact source-only export,
doc-check and performance-contract-check0.74 (local, non-promotable). A final
targeted controls/syntax check also confirms the permanent execution seal works in
a fresh checkout without relying on a target-directory STOP file. No resource
observation, candidate execution or Root full gate was repeated after rejection.

The packet retains92 members/50 objects, including original first-final producer,
current sources and complete original Job outcomes. Data-only verification:

```powershell
python -B tools/chitchat-fused-result-074/retain_verification.py verify ba395cf60db35db3de930de8db8732ab9361fcc9b8600023d1163add4019e9bc
```

F remains declined: returned Vec capacity B256/F268 for155 identical wire bytes.
No full allocation/CPU/RSS qualification, Root adoption or release/server admission.
