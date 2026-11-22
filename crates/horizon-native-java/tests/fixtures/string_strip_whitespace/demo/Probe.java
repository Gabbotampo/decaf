package demo;
public final class Probe {
  static final char[] HEX = {
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'
  };

  static String hx(int cp) {
    char[] out = new char[6];
    for (int i = 5; i >= 0; i--) {
      out[i] = HEX[cp & 15];
      cp >>= 4;
    }
    return new String(out);
  }

  /** The code point as a string (one char in the BMP, a surrogate pair otherwise). */
  static String chr(int cp) {
    return Character.toString(cp);
  }

  static String units(String s) {
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < s.length(); i++) {
      out.append(hx(s.charAt(i)));
    }
    return out.toString();
  }

  static String lenUnits(String s) {
    return "len=" + s.length() + " units=" + units(s);
  }

  static String row(String label, String value) {
    return label + "=" + value + "\n";
  }

  /**
   * The code points the framed rows are built for, ascending: every code point
   * {@code Character.isWhitespace} accepts, plus U+0085, U+00A0, U+180E, U+2007,
   * U+200B, U+202F and U+FEFF, which Java's strip keeps although some of them are
   * Unicode White_Space (U+0085, U+00A0, U+2007, U+202F) or look blank.
   * {@link #sets()} checks the list against the whole code point range.
   */
  static final int[] FRAMED = {
    0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x001C, 0x001D, 0x001E, 0x001F, 0x0020,
    0x0085, 0x00A0, 0x1680, 0x180E,
    0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A,
    0x200B, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000, 0xFEFF,
  };

  static boolean special(int cp) {
    return cp == 0x00A0 || cp == 0x2007 || cp == 0x202F || cp == 0x200B || cp == 0xFEFF
        || cp == 0x0085 || cp == 0x180E;
  }

  static void check(boolean ok, String what, int cp) {
    if (!ok) {
      throw new IllegalStateException(what + " at U+" + hx(cp));
    }
  }

  /**
   * The whole-range scans, run on the JDK only (the test measures the runtime
   * against these rows without repeating the scan): one {@code wsXXXXXX=1} row
   * per {@code Character.isWhitespace} code point, one {@code spXXXXXX=1} row per
   * {@code Character.isSpaceChar} code point. It also throws unless, for every
   * code point, {@code strip}, {@code stripLeading}, {@code stripTrailing} and
   * {@code isBlank} treat the code point as whitespace exactly when
   * {@code Character.isWhitespace} does (checked on {@code cp}, {@code "a"+cp+"b"}
   * and {@code cp+"a"+cp}), and unless {@link #FRAMED} is the ascending union of
   * the whitespace set and the specials.
   */
  static String sets() {
    StringBuilder out = new StringBuilder();
    int framed = 0;
    for (int cp = 0; cp <= 0x10FFFF; cp++) {
      boolean ws = Character.isWhitespace(cp);
      boolean inList = framed < FRAMED.length && FRAMED[framed] == cp;
      check(inList == (ws || special(cp)), "FRAMED differs from whitespace + specials", cp);
      if (inList) {
        framed++;
      }
      String c = chr(cp);
      check(c.isBlank() == ws, "isBlank", cp);
      check((c + "a" + c).strip().equals("a") == ws, "strip", cp);
      check((c + "a" + c).stripLeading().equals(ws ? "a" + c : c + "a" + c), "stripLeading", cp);
      check((c + "a" + c).stripTrailing().equals(ws ? c + "a" : c + "a" + c), "stripTrailing", cp);
      check(("a" + c + "b").strip().equals("a" + c + "b"), "strip interior", cp);
      if (ws) {
        out.append(row("ws" + hx(cp), "1"));
      }
    }
    check(framed == FRAMED.length, "FRAMED has code points out of range", 0);
    for (int cp = 0; cp <= 0x10FFFF; cp++) {
      if (Character.isSpaceChar(cp)) {
        out.append(row("sp" + hx(cp), "1"));
      }
    }
    return out.toString();
  }

  /** The framed rows: every method on each {@link #FRAMED} code point in three subjects. */
  public static String rows() {
    StringBuilder out = new StringBuilder();
    for (int k = 0; k < FRAMED.length; k++) {
      int cp = FRAMED[k];
      String c = chr(cp);
      String id = hx(cp);
      String[] subjects = { "a" + c + "b", c + "a" + c, c };
      String[] names = { "a" + id, "b" + id, "z" + id };
      for (int j = 0; j < 3; j++) {
        String subject = subjects[j];
        String name = names[j];
        out.append(row(name + ".strip", lenUnits(subject.strip())));
        out.append(row(name + ".stripLeading", lenUnits(subject.stripLeading())));
        out.append(row(name + ".stripTrailing", lenUnits(subject.stripTrailing())));
        out.append(row(name + ".trim", lenUnits(subject.trim())));
        out.append(row(name + ".isBlank", subject.isBlank() ? "true" : "false"));
      }
    }
    return out.toString();
  }

  public static void main(String[] args) {
    System.out.print(sets() + rows());
  }
}
