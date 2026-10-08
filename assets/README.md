# Application icon

`todotxt.png` is the 32-pixel frame of the upstream
`Client/TodoTouch_512.ico`, converted with System.Drawing. It is reused under the
upstream BSD license; see the root `LICENSE`. No upstream source was changed.
`todotxt.ico` is the unmodified upstream ICO used in the Windows executable.
`windows.rc` and `windows.manifest` embed that icon and enable themed native
controls, per-monitor DPI awareness, and ordinary non-admin execution.
