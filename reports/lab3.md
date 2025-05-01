# rCore 教程第 5 章实验报告

## 成果简述

本章实验内容相对简单，不过处理 `stride` 的比较，以及 `spawn` 新任务后的调度有一些细节容易导致难以发现的 BUG．为此，在 TaskControlBlock 上记录了其对应的程序，这样在很大程度上有助于调试内核．

## 问答题

### Stride 算法

1. 实际上还是 $p2$ 执行，因为整形溢出导致 $tride_{p2} = 250 + 20 (\mod 256) = 14 < 255 = stride_{p1}$．
2. 算法中 $pass_{min} = 1$ 而 $pass_{max} = BigStride / 2$．对于任意两个任务 $a$ 和 $b$，当 $stride_a = stride_b$ 时，其一步进后有 $| stride_a - stride_b | <= pass_{max}$；而当 $stride_a \ne stride_b$ 时，其一步进后有 $| stride_a - stride_b | < pass_{max}$．综上所述，在算法运行期间恒有 $| stride_a - stride_b | <= BigStride / 2$.
3. 代码如下：

```rust
#[derive(Eq)]
struct Stride(usize);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.cmp(&other.0) {
            Ordering::Less if (other.0 - self.0) > 127 => Ordering::Greater,
            Ordering::Greater if (self.0 - other.0) > 127 => Ordering::Less,
            other => other,
        }
        .reverse() // Rust's BinaryHeap is a max-heap, so we need to reverse the ordering to force a min-heap.
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
```

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

> 无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

> 无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
