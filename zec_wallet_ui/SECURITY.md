# Security

`zec_wallet` holds and spends real funds. It has not had an independent
security audit yet. Use it with amounts you can afford to lose until one is
published.

## Reporting a vulnerability

Email **security@relim.io** with a description and, if you can, steps to
reproduce. Please do not open a public issue for a vulnerability. You will get
an answer within a few days, and a fix is released before the details are
made public.

Keys never enter Dart. We treat as critical any report that shows key
material crossing into Dart, a payment that differs from what the user
approved, or funds that can be stranded.
