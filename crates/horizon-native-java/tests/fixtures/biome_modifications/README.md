Java 25 capture:
`J=~/ava/jdk/jdk-25.0.5+8/Contents/Home/bin; L=~/ava/scratch/vanilla-values/oracle/libraries/server; CP=~/ava/scratch/issue-476/biome.jar:~/ava/minecraft-26.3-server-game.jar:$(find $L -name "*.jar" | tr "\\n" ":"); $J/javac -cp "$CP" -d ~/ava/scratch/issue-476 demo/Probe.java; $J/java -cp "$CP:$HOME/ava/scratch/issue-476" demo.Probe`
