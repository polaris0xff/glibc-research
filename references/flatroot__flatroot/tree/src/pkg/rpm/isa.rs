//! Recognizes and strips RPM's instruction-set (ISA) qualifier on a
//! dependency name — meaningless in a single-architecture index —
//! without disturbing the similar-looking soname symbol-version or
//! ELF-class markers.

/// Recognizes and strips RPM ISA qualifiers, sharing one definition so
/// the two never disagree about which parenthesized groups qualify.
pub(super) struct IsaQualifier;

impl IsaQualifier {
  /// Strips the ISA qualifier (`glibc(x86-64)` → `glibc`) from a
  /// dependency name, leaving the soname symbol-version
  /// (`(GLIBC_2.34)`) and ELF-class (`(64bit)`) groups untouched.
  pub(super) fn arch_strip(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(open) = rest.find('(') {
      let Some(close_rel) = rest[open..].find(')') else {
        break; // unbalanced '(' — keep the remainder verbatim
      };
      let close = open + close_rel;
      if IsaQualifier::matches(&rest[open + 1..close]) {
        out.push_str(&rest[..open]);
      } else {
        out.push_str(&rest[..=close]);
      }
      rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
  }

  /// Whether a parenthesised group is an ISA qualifier (`x86-64`,
  /// `aarch64`, …) rather than a symbol-version (`GLIBC_2.34`) or ELF-class
  /// (`64bit`) marker: a lowercase architecture token ending in `32` or
  /// `64`.
  pub(super) fn matches(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    !inner.is_empty()
      && bytes[0].is_ascii_lowercase()
      && bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-' || *b == b'_')
      && (inner.ends_with("32") || inner.ends_with("64"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn arch_qualifier_strip_drops_isa_and_keeps_soname_markers() {
    // ISA qualifiers are removed from a plain package name.
    assert_eq!(IsaQualifier::arch_strip("glibc(x86-64)"), "glibc");
    assert_eq!(IsaQualifier::arch_strip("glibc(x86-32)"), "glibc");
    assert_eq!(IsaQualifier::arch_strip("libfoo(aarch64)"), "libfoo");
    assert_eq!(IsaQualifier::arch_strip("libgomp(ppc-64)"), "libgomp");
    // A trailing version constraint after the qualifier survives.
    assert_eq!(IsaQualifier::arch_strip("glibc(x86-64) >= 2.34"), "glibc >= 2.34");
    // Soname symbol-version and ELF-class markers are not ISA qualifiers.
    assert_eq!(IsaQualifier::arch_strip("libc.so.6()(64bit)"), "libc.so.6()(64bit)");
    assert_eq!(IsaQualifier::arch_strip("libc.so.6(GLIBC_2.34)(64bit)"), "libc.so.6(GLIBC_2.34)(64bit)");
    // A non-ISA virtual provide and a plain name are left untouched.
    assert_eq!(IsaQualifier::arch_strip("python(abi)"), "python(abi)");
    assert_eq!(IsaQualifier::arch_strip("bash"), "bash");
  }

  #[test]
  fn isa_qualifier_matches_distinguishes_isa_from_markers() {
    assert!(IsaQualifier::matches("x86-64"));
    assert!(IsaQualifier::matches("x86-32"));
    assert!(IsaQualifier::matches("aarch64"));
    assert!(IsaQualifier::matches("aarch-64"));
    assert!(IsaQualifier::matches("riscv-64"));
    assert!(!IsaQualifier::matches("64bit"));
    assert!(!IsaQualifier::matches("GLIBC_2.34"));
    assert!(!IsaQualifier::matches("abi"));
    assert!(!IsaQualifier::matches(""));
  }
}
