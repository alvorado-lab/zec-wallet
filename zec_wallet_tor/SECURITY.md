# Security

`zec_wallet_tor` carries the network traffic of a wallet that holds and
spends real funds. It has not had an independent security audit yet. Use it
with amounts you can afford to lose until one is published.

## Reporting a vulnerability

Email **security@relim.io** with a description and, if you can, steps to
reproduce. Please do not open a public issue for a vulnerability. You will get
an answer within a few days, and a fix is released before the details are
made public.

Treated as critical: a wallet set to `TorPolicy.required_` that connects
without Tor; a connection that leaves the device outside Tor while the plugin
reports it ready; the bridge lines, the Tor directory path or guard identities
appearing in a log; and Tor state surviving `clearState`.
