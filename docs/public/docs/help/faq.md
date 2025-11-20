# Frequently asked questions

## **Can I run inside a docker container?**

You can run inside docker but the application will not be able to reach any keyring to store safely any encryption key. You will need to use the local file system to store your encryption keys which is unsafe. To use the local file system:

 1. Ensure you are logged out by doing `pass-cli logout --force`
 2. Set the environment variable `PROTON_PASS_KEY_PROVIDER` to `fs`.
 3. Login normally as you would.

## **Do you send any telemetry?**

We send anonimized telemetry that **never** includes any personal or sensitive data. It only sends what action was done like `item created of type note` with client `X` but **never** send any contents or anything that can be used to track any data or user. We use this information to try to make the product better.

## **Can I disable telemetry?**

Certainly! There are many ways of disabling it. If you want to disable it for this application you can set an environment variable `PROTON_PASS_DISABLE_TELEMETRY`. If the environment variable is set telemetry will not be saved and the currently saved locally will be cleared.

If you want to disable it globally you can go to your [Account security settings](https://account.proton.me/pass/security) and disable `Collect usage diagnostics`
