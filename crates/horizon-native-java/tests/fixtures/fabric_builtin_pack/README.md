Oracle facts for Fabric ResourceLoader.registerBuiltinPack. Fabric cannot run outside a
Minecraft loader, so these are the Java 25 javap facts used by the test.

```sh
. ~/ava/env.sh
J=~/ava/jdk/jdk-25.0.5+8/Contents/Home/bin
M=~/ava/scratch-register.jar
$J/javap -c -p -classpath "$M" net.fabricmc.fabric.api.resource.v1.ResourceLoader
$J/javap -c -p -classpath "$M" net.fabricmc.fabric.impl.resource.ResourceLoaderImpl
$J/javap -c -p -classpath "$M" net.fabricmc.fabric.impl.resource.pack.ModNioPackResources
```
