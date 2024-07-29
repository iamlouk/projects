target datalayout = "e-m:e-p:64:64-i64:64-i128:128-n32:64-S128"
target triple = "riscv64-unknown-linux-gnu"

define i64 @sum(i64 noundef %N, ptr %A) #0 {
entry:
  %guard.cond = icmp eq i64 %N, 0
  br i1 %guard.cond, label %exit, label %loop

loop:
  %i = phi i64 [ %i.next, %loop ], [ 0, %entry ]
  %sum.phi = phi i64 [ %sum.next, %loop ], [ 0, %entry ]
  %gep = getelementptr inbounds i64, ptr %A, i64 %i
  %a.val = load i64, ptr %gep, align 8
  %sum.next = add i64 %sum.phi, %a.val
  %i.next = add nuw i64 %i, 1
  %exit.cond = icmp eq i64 %i.next, %N
  br i1 %exit.cond, label %exit, label %loop

exit:
  %sum.final = phi i64 [ 0, %entry ], [ %sum.next, %loop ]
  ret i64 %sum.final
}

attributes #0 = { "target-cpu"="generic-rv64" "target-features"="+64bit" }
