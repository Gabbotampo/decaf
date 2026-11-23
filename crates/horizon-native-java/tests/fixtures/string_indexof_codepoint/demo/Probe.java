package demo;

public final class Probe {
    public static String run() {
        StringBuilder out = new StringBuilder();

        String abc = "abc";
        String pair = new String(new char[] {'a', 'b', '\uD83D', '\uDE00', 'c', 'd'});
        String empty = "";

        int[] needles = {-1, Integer.MIN_VALUE, 0, 0x61, 0xFFFF, 0xD83D, 0xDE00,
                0x1F600, 0x10FFFF, 0x110000, Integer.MAX_VALUE};

        // Single-arg indexOf(int)
        for (String s : new String[] {abc, pair, empty}) {
            for (int n : needles) {
                out.append("I|").append(n).append('|').append(escape(s)).append('=')
                    .append(s.indexOf(n)).append('\n');
            }
        }

        // Single-arg lastIndexOf(int)
        for (String s : new String[] {abc, pair, empty}) {
            for (int n : needles) {
                out.append("LI|").append(n).append('|').append(escape(s)).append('=')
                    .append(s.lastIndexOf(n)).append('\n');
            }
        }

        // Two-arg indexOf(int,int) on "abc"
        for (int n : new int[] {-1, 0, 0x61, 0xFFFF, 0xD83D, 0x1F600, 0x110000}) {
            for (int f : new int[] {-5, 0, 1, abc.length(), abc.length() + 5,
                    Integer.MIN_VALUE, Integer.MAX_VALUE}) {
                out.append("II|").append(n).append('|').append(f).append('|').append(escape(abc)).append('=')
                    .append(abc.indexOf(n, f)).append('\n');
            }
        }

        // Two-arg indexOf(int,int) on empty string
        for (int n : new int[] {-1, 0, 0x61, 0x1F600}) {
            for (int f : new int[] {-5, 0, 1, empty.length(), empty.length() + 5,
                    Integer.MIN_VALUE, Integer.MAX_VALUE}) {
                out.append("II|").append(n).append('|').append(f).append('|').append(escape(empty)).append('=')
                    .append(empty.indexOf(n, f)).append('\n');
            }
        }

        // Two-arg lastIndexOf(int,int) on "abc"
        for (int n : new int[] {-1, 0, 0x61, 0xFFFF, 0xD83D, 0x1F600, 0x110000}) {
            for (int f : new int[] {-5, 0, 1, abc.length(), abc.length() + 5,
                    Integer.MIN_VALUE, Integer.MAX_VALUE}) {
                out.append("LII|").append(n).append('|').append(f).append('|').append(escape(abc)).append('=')
                    .append(abc.lastIndexOf(n, f)).append('\n');
            }
        }

        // Two-arg lastIndexOf(int,int) on empty string
        for (int n : new int[] {-1, 0, 0x61, 0x1F600}) {
            for (int f : new int[] {-5, 0, 1, empty.length(), empty.length() + 5,
                    Integer.MIN_VALUE, Integer.MAX_VALUE}) {
                out.append("LII|").append(n).append('|').append(f).append('|').append(escape(empty)).append('=')
                    .append(empty.lastIndexOf(n, f)).append('\n');
            }
        }

        // Two-arg indexOf/lastIndexOf(int,int) on a string holding a surrogate pair
        for (int n : new int[] {-1, 0x61, 0x63, 0xD83D, 0xDE00, 0x1F600, 0x110000}) {
            for (int f = -2; f <= pair.length() + 2; f++) {
                out.append("IIP|").append(n).append('|').append(f).append('|').append(escape(pair)).append('=')
                    .append(pair.indexOf(n, f)).append('\n');
                out.append("LIIP|").append(n).append('|').append(f).append('|').append(escape(pair)).append('=')
                    .append(pair.lastIndexOf(n, f)).append('\n');
            }
        }

        return out.toString();
    }

    public static void main(String[] args) {
        System.out.print(run());
    }

    static String escape(String s) {
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '\uD83D') {
                b.append("\\uD83D");
            } else if (c == '\uDE00') {
                b.append("\\uDE00");
            } else {
                b.append(c);
            }
        }
        return b.toString();
    }
}
