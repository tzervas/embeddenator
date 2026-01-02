# Unwrap/Expect Quick Reference

**For:** Development team maintaining embeddenator  
**Purpose:** Guidelines for when to use unwrap/expect safely

---

## 🚦 Decision Tree

```
Is this test code?
├─ YES → ✅ unwrap() is OK
└─ NO
   └─ Is this a doc comment example?
      ├─ YES → ✅ unwrap() is OK  
      └─ NO
         └─ Is failure truly impossible?
            ├─ YES → Use expect() with SAFETY comment
            └─ NO → ❌ Return Result/Option instead
```

---

## ✅ When Unwrap/Expect is Acceptable

### 1. Test Code
```rust
#[test]
fn test_something() {
    let result = operation().unwrap();  // ✅ OK
    assert_eq!(result, expected);
}
```

### 2. Doc Comments
```rust
/// # Examples
/// ```
/// let fs = EmbrFS::new(false);
/// fs.add_file("/test.txt", data).unwrap();  // ✅ OK
/// ```
```

### 3. Provably Safe (with SAFETY comment)
```rust
// ✅ OK - key from keys()
for id in map.keys() {
    // SAFETY: id comes from keys(), so get() must succeed
    let value = map.get(id).expect("key from keys()");
}

// ✅ OK - length checked
if vec.len() == 1 {
    // SAFETY: we just checked len() == 1
    let item = vec.into_iter().next().expect("vec has one element");
}

// ✅ OK - cryptographic guarantee
let hash = Sha256::hash(data);
// SAFETY: SHA256 always produces 32 bytes
let bytes: [u8; 32] = hash[..].try_into().expect("SHA256 is 32 bytes");
```

---

## ❌ When to Return Result/Option Instead

### 1. External Input
```rust
// ❌ BAD - user input can be anything
let config: Config = serde_json::from_str(json).unwrap();

// ✅ GOOD
let config: Config = serde_json::from_str(json)?;
```

### 2. File/Network Operations
```rust
// ❌ BAD - file might not exist
let data = fs::read("config.toml").unwrap();

// ✅ GOOD
let data = fs::read("config.toml")?;
```

### 3. Parsing/Conversion
```rust
// ❌ BAD - string might not be valid number
let num: i32 = s.parse().unwrap();

// ✅ GOOD
let num: i32 = s.parse()?;
```

### 4. Collection Access
```rust
// ❌ BAD - index might be out of bounds
let item = vec[index];

// ✅ GOOD
let item = vec.get(index).ok_or_else(|| anyhow!("Index {} out of bounds", index))?;
```

### 5. Float Comparisons
```rust
// ❌ BAD - returns None for NaN
items.sort_by(|a, b| a.partial_cmp(b).unwrap());

// ✅ GOOD - handle NaN gracefully
items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
```

---

## 📝 SAFETY Comment Guidelines

### Required Elements

1. **Use `// SAFETY:`** (not `// Safe:`)
2. **Explain the invariant** that makes it safe
3. **Reference the guard** that establishes the invariant

### Good Examples

```rust
// ✅ GOOD - explains WHY it's safe
if map.contains_key(&key) {
    // SAFETY: we just verified key exists via contains_key()
    let value = map.get(&key).expect("key exists");
}

// ✅ GOOD - references cryptographic property
// SAFETY: SHA256 always produces 32 bytes by specification
let hash: [u8; 32] = hasher.finalize()[..].try_into().expect("SHA256 is 32 bytes");

// ✅ GOOD - references function precondition
// SAFETY: process_batch() is only called with non-empty vectors
let first = items.first().expect("items is non-empty per precondition");
```

### Bad Examples

```rust
// ❌ BAD - doesn't explain WHY
let value = map.get(&key).unwrap(); // this is safe

// ❌ BAD - no comment at all
let value = map.get(&key).unwrap();

// ❌ BAD - wrong marker style
// Safe: key exists
let value = map.get(&key).unwrap();
```

---

## 🔍 Code Review Checklist

When reviewing code with `unwrap()` or `expect()`:

- [ ] Is it in test code or doc comments? → OK
- [ ] Is there a `// SAFETY:` comment? → Check invariant
- [ ] Does the invariant actually hold? → Verify logic
- [ ] Could this panic in production? → Request Result/Option
- [ ] Is the expect message descriptive? → Should explain what failed

---

## 🛠️ Refactoring Patterns

### Pattern 1: Option Chain
```rust
// Before
let value = map.get(&key).unwrap();
process(value);

// After
let value = map.get(&key)?;
process(value);
```

### Pattern 2: Early Return
```rust
// Before
if condition {
    return Err(...);
}
let value = option.unwrap();

// After (if truly safe)
if condition {
    return Err(...);
}
// SAFETY: condition check above guarantees option is Some
let value = option.expect("option is Some after condition check");

// Or just use ? if you can modify function signature
let value = option.ok_or_else(|| anyhow!("..."))?;
```

### Pattern 3: Graceful Degradation
```rust
// Before - panics on NaN
similarities.sort_by(|a, b| a.partial_cmp(b).unwrap());

// After - treats NaN as smallest
similarities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
```

---

## 📊 Current Project Status

| Category | Count | Status |
|----------|-------|--------|
| Test unwraps | 15 | ✅ Allowed |
| Doc unwraps | 11 | ✅ Allowed |
| Safe expects | 11 | ✅ Documented |
| Fixed issues | 1 | ✅ Resolved |
| **Production risks** | **0** | **✅ None** |

---

## 🔗 Related Documents

- **UNWRAP_AUDIT_COMPLETE.md** - Full audit report with statistics
- **UNWRAP_FIXES_BEFORE_AFTER.md** - Detailed before/after code examples
- **UNWRAP_FIXES_SUMMARY.md** (previous) - Earlier audit work

---

## 📚 Further Reading

**Rust Documentation:**
- [The Rust Book - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Rust API Guidelines - Error Handling](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)
- [std::result::Result](https://doc.rust-lang.org/std/result/)
- [std::option::Option](https://doc.rust-lang.org/std/option/)

**Best Practices:**
- [Rust Error Handling Patterns](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [The Rust RFC on Safety Comments](https://rust-lang.github.io/rfcs/2585-unsafe-block-in-unsafe-fn.html)

---

**Last Updated:** 2026-01-02  
**Version:** 1.0  
**Maintainer:** Development Team
