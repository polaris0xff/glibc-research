# images/extra-ca/

Drop a `.crt` (PEM) here and every image build appends it to the image's trust
store before any package manager or `curl` runs.

**Why this exists.** Some networks terminate TLS on the way out: corporate
egress proxies, some CI gateways, and sandboxed agent environments all do it.
Inside such a network a container build fails with `certificate verify failed`
on the first `apk`/`apt`/`pacman` call, and the usual "fix" people reach for is
to disable verification. ⛔ That is never done here. Supplying the CA is.

```sh
cp /path/to/corporate-ca.pem images/extra-ca/proxy.crt
export ALLOC_TESTS_HTTPS_PROXY=http://proxy.internal:3128   # if one is needed
alloc-bench run --suite smoke
```

The directory is empty by default and the build step is a no-op when it holds
no `.crt`, so nothing changes for a normal network.

⚠ A certificate here is trusted by everything the image builds. Only put one in
that your organisation already requires you to trust.
