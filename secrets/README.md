# RedNode Secrets – sops + age

Generate key:
```
age-keygen -o age.key
# add public key to .sops.yaml
```

Encrypt:
```
sops -e secrets/rednode.secrets.yaml > secrets/rednode.secrets.enc.yaml
```

Decrypt at runtime:
```
sops -d secrets/rednode.secrets.enc.yaml
```

No Vault daemon required – perfect for portable RedNode.
