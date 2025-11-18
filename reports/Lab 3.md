# `Lab 3`

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

------

## 问答题解答

### 问题1：实际情况是轮到 p1 执行吗？为什么？

不是。实际情况是 p2 会再次执行。

**原因**：
- p1.stride = 255，p2.stride = 250
- p2 执行一个时间片后，stride 增加 pass=10，得到 250+10=260
- 由于使用 8bit 无符号整数，260 会溢出变成 4（260 mod 256 = 4）
- 此时 p1.stride = 255，p2.stride = 4
- 由于 4 < 255，所以 p2 会再次被执行，而不是 p1

### 问题2：为什么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2？

**原因**：
- 进程优先级 ≥ 2，所以每个进程的 pass = BigStride / priority ≤ BigStride / 2
- 当某个进程的 stride 落后其他进程超过 BigStride / 2 时，它会被立即调度
- 因为它的 stride 最小，会被调度器选中
- 调度后它的 stride 增加 pass ≤ BigStride / 2，仍然不会超过其他进程的 stride 超过 BigStride / 2
- 因此，所有进程的 stride 差值始终不会超过 BigStride / 2

### 问题3：补全比较器代码

```rust
use core::cmp::Ordering;

const BIG_STRIDE: u64 = 255; // 假设 BigStride 为 255

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.0;
        let b = other.0;
        let diff = (a as i16).wrapping_sub(b as i16);
        
        if diff > 0 {
            // a > b
            if diff <= (BIG_STRIDE / 2) as i16 {
                Some(Ordering::Greater)
            } else {
                // 差值过大，说明 b 溢出了，实际 b > a
                Some(Ordering::Less)
            }
        } else {
            // a < b
            if (-diff) <= (BIG_STRIDE / 2) as i16 {
                Some(Ordering::Less)
            } else {
                // 差值过大，说明 a 溢出了，实际 a > b
                Some(Ordering::Greater)
            }
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

**原理**：
- 利用有符号整数的环绕减法计算差值
- 如果差值的绝对值 ≤ BigStride / 2，说明没有跨越溢出边界，直接比较
- 如果差值的绝对值 > BigStride / 2，说明跨越了溢出边界，实际大小关系与表面相反

## 功能总结

我实现了两个系统调用：

1. **`sys_set_priority(prio: isize) -> isize`**：
   - 设置当前进程的优先级
   - 优先级必须 ≥ 2，否则返回 -1
   - 设置成功后重新计算步长 pass = BIG_STRIDE / priority
   - 返回设置的优先级

2. **`sys_spawn(path: *const u8) -> isize`**：
   - 创建新的子进程执行指定程序
   - 不复制父进程地址空间，直接加载目标程序
   - 建立正确的父子进程关系
   - 设置子进程返回值为 0
   - 成功返回子进程 PID，失败返回 -1

同时实现了 stride 调度算法，为每个进程维护 stride、pass、priority 字段，调度时选择 stride 最小的进程，确保高优先级进程获得更多 CPU 时间。