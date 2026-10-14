# Public synthetic OIDC TLS fixtures

The three `public-synthetic-oidc-tls-*.pem` files are intentionally public test
data, not credentials. The RSA server private key is generated solely for owned
loopback tests and cannot authenticate any production service under platform
trust. Never install the synthetic CA in a system trust store or use these files
for signing, deployments, protected GitHub jobs or host enrollment.

The synthetic server certificate names the fixed test target used by the client;
only a private test context trusts its synthetic CA. Its fixed 2020–2040 validity
supports repeatable local TLS tests, not a production-key lifetime policy. The CA
private key was not retained. No production key or GitHub token was used to
generate the fixtures.
