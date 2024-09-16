
This project uses [libelfin](https://github.com/aclements/libelfin), a ELF and DWARF parser library. It only supports DWARF up to version 4, so one needs to compile code with `-gdwarf-4`. The patch below helps a bit by allowing libraries or object files with DWARF version 5 to be mixed into the binary. Parsing ELF is not that hard, I wrote a simple ELF parser myself, but DWARF is a big mess and that library is great!

```diff
diff --git a/dwarf/dwarf.cc b/dwarf/dwarf.cc
index 2465eef..eeab804 100644
--- a/dwarf/dwarf.cc
+++ b/dwarf/dwarf.cc
@@ -68,8 +68,12 @@ dwarf::dwarf(const std::shared_ptr<loader> &l)
                 // XXX Circular reference.  Given that we now require
                 // the dwarf object to stick around for DIEs, maybe we
                 // might as well require that for units, too.
-                m->compilation_units.emplace_back(
-                        *this, infocur.get_section_offset());
+                try {
+                        m->compilation_units.emplace_back(
+                                *this, infocur.get_section_offset());
+                } catch (const format_error &e) {
+                        // Whatever, ignore this...
+                }
                 infocur.subsection();
         }
 }
```

