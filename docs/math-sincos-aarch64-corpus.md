# AArch64 `Math.sin`/`Math.cos` corpus

Temurin 25.0.5+8 on arcweld (AArch64), deterministic LCG seed `0x3141592653589793`, 1,048,576 raw-bit inputs per function. `-Xint`, `-Xbatch -XX:TieredStopAtLevel=1`, and `-Xbatch -XX:TieredStopAtLevel=4` agreed with the native AArch64 fdlibm/FMA path with **0 mismatches**.

Commands: `java $mode CorpusGen > java.bin`; native probe output to `rust.bin`; compare each raw-bit `(sin,cos)` row. Input SHA-256: `60be8557b5c9a8461a82fe7852810ed7dbef166c41cdb1126376bc6206e32ccd`. Last-tier Java output SHA-256: `1ccfc15bffc1f12f32e4d5f2c8cb57a92d868180fd234947bc99db74021fc374`; native output SHA-256: `f3ffa9b91464ecb7a0fd58f928b2c3a621ffa0e212aa7cb1bf06ced0d68054f9`.

Samples (`input sin cos`): `0056bafb6dbb1326 3ff0000000000000 3ff0000000000000`; `e6c4110d7079a881 3fef84e405049bb1 bfc61b7e6695bf7e`; `7fefffffffffffff 3f7452fc98b34e97 bfefffe62ecfab76`; `cece41e9ea689793 bfc7adc8fa1251f7 3fef729b9b7bdb38`.
