; cmd: make && opt -S -load-pass-plugin=./libShittyCSE.so -passes=shitty-cse ../test.ll -o -

define i32 @foo(i32 %a, i32 %b) {
entry:
  %square1 = mul i32 %a, %b
  %square2 = mul i32 %a, %b
  %add = add i32 %square1, %square2
  ret i32 %add
}

define i32 @bar(i32 %a, i32 %b) {
entry:
  %square1 = mul i32 %a, %b
  %square2 = mul i32 %b, %a
  %add = add i32 %square1, %square2
  ret i32 %add
}

define i32 @baz(i32 %a, i32 %b, i32 %c, i1 %cond) {
entry:
  %square1 = mul i32 %a, %a
  br i1 %cond, label %left, label %right

left:
  %otheradd = add i32 %square1, %square1
  br label %join

right:
  br label %join

join:
  %square2 = mul i32 %a, %a
  %add = add i32 %square1, %square2
  ret i32 %add
}

