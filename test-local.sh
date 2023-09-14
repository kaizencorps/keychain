# test against locally running validator - needs to have already been deployed & anchor.toml set to cluster = 'localnet'
anchor test --skip-local-validator --skip-deploy --skip-build
