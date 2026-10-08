# Security

The packages in this repository hold and spend real funds. They have not had
an independent security audit yet. Use them with amounts you can afford to
lose until one is published.

## Reporting a vulnerability

Email **security@relim.io** with a description and, if you can, steps to
reproduce. Please do not open a public issue for a vulnerability. You will get
an answer within a few days, and a fix is released before the details are
made public.

Keys never enter Dart. We treat as critical any report that shows key
material crossing into Dart, a payment that differs from what the user
approved, or funds that can be stranded.

For the optional Tor plugin (`zec_wallet_tor`, `dialer-tor`), these are
critical too: a wallet set to `TorPolicy.required_` that connects without Tor; a
connection that leaves the device outside Tor while the plugin reports it
ready; the bridge lines, the Tor directory path or guard identities appearing
in a log; and Tor state surviving `clearState`.
