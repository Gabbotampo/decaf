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

  public static String rows() {
    StringBuilder out = new StringBuilder();
    // The whitespace set: every code point Character.isWhitespace answers true for.
    for (int cp = 0; cp <= 0x10FFFF; cp++) {
      if (Character.isWhitespace(cp)) {
        out.append(row("ws" + hx(cp), "1"));
      }
    }
    // The space-char set: every code point Character.isSpaceChar answers true for.
    for (int cp = 0; cp <= 0x10FFFF; cp++) {
      if (Character.isSpaceChar(cp)) {
        out.append(row("sp" + hx(cp), "1"));
      }
    }
    // Every whitespace code point, plus the characters Java's strip keeps
    // (U+00A0, U+2007, U+202F) and a few others (U+200B, U+FEFF, U+0085,
    // U+180E), each framed by letters in three subjects.
    int[] specials = { 0x00A0, 0x2007, 0x202F, 0x200B, 0xFEFF, 0x0085, 0x180E };
    for (int cp = 0; cp <= 0x10FFFF; cp++) {
      boolean special = false;
      for (int s : specials) {
        if (s == cp) {
          special = true;
          break;
        }
      }
      if (!Character.isWhitespace(cp) && !special) {
        continue;
      }
      String c = chr(cp);
      String id = hx(cp);
      String[] subjects = { "a" + c + "b", c + "a" + c, c };
      String[] names = { "a" + id, "b" + id, "z" + id };
      for (int k = 0; k < 3; k++) {
        String subject = subjects[k];
        String name = names[k];
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
    System.out.print(rows());
  }
}
