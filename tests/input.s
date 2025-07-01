.globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    subq $32, %rsp
    movl $0, %r11d
    cmpl $0, %r11d
    je .L_and_false_0
    movl $1, %r11d
    cmpl $0, %r11d
    je .L_and_false_0
    movl $1, -4(%rbp)
    jmp .L_and_end_1
.L_and_false_0:
    movl $0, -4(%rbp)
.L_and_end_1:
    movl -4(%rbp), %r10d
    movl %r10d, -8(%rbp)
    notl -8(%rbp)
    movl $4, %r11d
    cmpl $0, %r11d
    jne .L_or_true_2
    movl $3, %r11d
    cmpl $0, %r11d
    jne .L_or_true_2
    movl $0, -12(%rbp)
    jmp .L_or_end_3
.L_or_true_2:
    movl $1, -12(%rbp)
.L_or_end_3:
    movl -12(%rbp), %r10d
    movl %r10d, -16(%rbp)
    negl -16(%rbp)
    movl -8(%rbp), %r10d
    movl %r10d, -20(%rbp)
    movl -16(%rbp), %r10d
    subl %r10d, -20(%rbp)
    movl -20(%rbp), %eax
    movq %rbp, %rsp
    popq %rbp
    ret
.section .note.GNU-stack,"",@progbits
