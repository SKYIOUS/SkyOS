import os

for root, _, files in os.walk("coreutils/src"):
    for f in files:
        if f.endswith(".rs"):
            p = os.path.join(root, f)
            with open(p, "r") as file:
                content = file.read()
            if "fn user_main() -> i32 {" in content:
                content = content.replace("}\n\nsarga_main!", "    0\n}\n\nsarga_main!")
                content = content.replace("}\nsarga_main!", "    0\n}\nsarga_main!")
                with open(p, "w") as file:
                    file.write(content)
