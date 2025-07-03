// src/semantics/loop_labeler.rs

use crate::ast::{checked, unchecked};
use crate::common::UniqueIdGenerator;

pub struct LoopLabeler<'a> {
    // 用于生成唯一的循环 ID
    id_generator: &'a mut UniqueIdGenerator,
    // 一个栈，保存当前嵌套的循环的 ID
    loop_id_stack: Vec<checked::LoopId>,
}

impl<'a> LoopLabeler<'a> {
    pub fn new(id_generator: &'a mut UniqueIdGenerator) -> Self {
        LoopLabeler {
            id_generator,
            loop_id_stack: Vec::new(),
        }
    }

    // 主入口函数：接收 unchecked::Program，返回 checked::Program
    pub fn label_program(&mut self, prog: unchecked::Program) -> Result<checked::Program, String> {
        let func = self.label_function(prog.function)?;
        Ok(checked::Program { function: func })
    }

    fn label_function(&mut self, func: unchecked::Function) -> Result<checked::Function, String> {
        let body = self.label_block(func.body)?;
        Ok(checked::Function {
            name: func.name,
            body,
        })
    }

    fn label_block(&mut self, block: unchecked::Block) -> Result<checked::Block, String> {
        let mut items = Vec::new();
        for item in block.blocks {
            items.push(self.label_block_item(item)?);
        }
        Ok(checked::Block { blocks: items })
    }

    fn label_block_item(
        &mut self,
        item: unchecked::BlockItem,
    ) -> Result<checked::BlockItem, String> {
        match item {
            unchecked::BlockItem::S(stmt) => Ok(checked::BlockItem::S(self.label_statement(stmt)?)),
            // Declaration 不变，直接移动
            unchecked::BlockItem::D(decl) => Ok(checked::BlockItem::D(decl)),
        }
    }

    // --- 【核心转换逻辑】 ---
    fn label_statement(
        &mut self,
        stmt: unchecked::Statement,
    ) -> Result<checked::Statement, String> {
        match stmt {
            // --- 循环语句 ---
            unchecked::Statement::For {
                init,
                condition,
                post,
                body,
            } => {
                let loop_id = self.id_generator.next();
                self.loop_id_stack.push(loop_id);

                // 递归转换所有子节点
                let checked_init = init
                    .map(|i| self.label_block_item(*i))
                    .transpose()?
                    .map(Box::new);
                let checked_body = Box::new(self.label_statement(*body)?);

                self.loop_id_stack.pop();

                Ok(checked::Statement::For {
                    init: checked_init,
                    condition, // Expression 不变
                    post,      // Expression 不变
                    body: checked_body,
                    id: loop_id,
                })
            }
            unchecked::Statement::While { condition, body } => {
                let loop_id = self.id_generator.next();
                self.loop_id_stack.push(loop_id);
                let checked_body = Box::new(self.label_statement(*body)?);
                self.loop_id_stack.pop();
                Ok(checked::Statement::While {
                    condition,
                    body: checked_body,
                    id: loop_id,
                })
            }
            unchecked::Statement::DoWhile { body, condition } => {
                let loop_id = self.id_generator.next();
                self.loop_id_stack.push(loop_id);
                let checked_body = Box::new(self.label_statement(*body)?);
                self.loop_id_stack.pop();
                Ok(checked::Statement::DoWhile {
                    body: checked_body,
                    condition,
                    id: loop_id,
                })
            }

            // --- 跳转语句 ---
            unchecked::Statement::Break => {
                if let Some(&target_id) = self.loop_id_stack.last() {
                    Ok(checked::Statement::Break { target_id })
                } else {
                    Err("'break' statement not in a loop".to_string())
                }
            }
            unchecked::Statement::Continue => {
                if let Some(&target_id) = self.loop_id_stack.last() {
                    Ok(checked::Statement::Continue { target_id })
                } else {
                    Err("'continue' statement not in a loop".to_string())
                }
            }

            // --- 非循环/跳转语句的直接转换 ---
            unchecked::Statement::Return(e) => Ok(checked::Statement::Return(e)),
            unchecked::Statement::Expression(e) => Ok(checked::Statement::Expression(e)),
            unchecked::Statement::Empty => Ok(checked::Statement::Empty),
            unchecked::Statement::Compound(b) => {
                Ok(checked::Statement::Compound(self.label_block(b)?))
            }
            unchecked::Statement::If {
                condition,
                then_stat,
                else_stat,
            } => {
                let checked_then = Box::new(self.label_statement(*then_stat)?);
                let checked_else = else_stat
                    .map(|s| self.label_statement(*s))
                    .transpose()?
                    .map(Box::new);
                Ok(checked::Statement::If {
                    condition,
                    then_stat: checked_then,
                    else_stat: checked_else,
                })
            }
        }
    }
}
// src/semantics/loop_labeler.rs (文件末尾)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::unchecked::*;
    use crate::common::UniqueIdGenerator;

    // 辅助函数，用于创建一个包含嵌套循环的 unchecked AST
    fn create_test_unchecked_ast() -> Program {
        // C 代码等价物:
        // int main(void) {
        //     while (1) {           // 循环 ID 0
        //         for (;;) {        // 循环 ID 1
        //             if (1) continue;
        //             break;
        //         }
        //         break;
        //     }
        //     return 0;
        // }

        Program {
            function: Function {
                name: "main".to_string(),
                body: Block {
                    blocks: vec![
                        BlockItem::S(Statement::While {
                            condition: Expression::Constant(1),
                            body: Box::new(Statement::Compound(Block {
                                blocks: vec![
                                    BlockItem::S(Statement::For {
                                        init: None,
                                        condition: None,
                                        post: None,
                                        body: Box::new(Statement::Compound(Block {
                                            blocks: vec![
                                                BlockItem::S(Statement::If {
                                                    condition: Expression::Constant(1),
                                                    then_stat: Box::new(Statement::Continue),
                                                    else_stat: None,
                                                }),
                                                BlockItem::S(Statement::Break),
                                            ],
                                        })),
                                    }),
                                    BlockItem::S(Statement::Break),
                                ],
                            })),
                        }),
                        BlockItem::S(Statement::Return(Expression::Constant(0))),
                    ],
                },
            },
        }
    }

    #[test]
    fn test_loop_labeling_success() {
        let unchecked_ast = create_test_unchecked_ast();
        let mut id_gen = UniqueIdGenerator::new();
        let mut labeler = LoopLabeler::new(&mut id_gen);

        let checked_ast = labeler
            .label_program(unchecked_ast)
            .expect("Labeling should succeed");

        // 断言顶层结构
        let main_body_items = &checked_ast.function.body.blocks;
        assert_eq!(main_body_items.len(), 2);

        // --- 深入检查外层 while 循环 (应该有 id=0) ---
        if let checked::BlockItem::S(checked::Statement::While {
            id: while_id,
            body: while_body,
            ..
        }) = &main_body_items[0]
        {
            assert_eq!(*while_id, 0, "Outer while loop should have id 0");

            if let checked::Statement::Compound(while_block) = &**while_body {
                let while_body_items = &while_block.blocks;
                assert_eq!(while_body_items.len(), 2);

                // --- 检查内层 for 循环 (应该有 id=1) ---
                if let checked::BlockItem::S(checked::Statement::For {
                    id: for_id,
                    body: for_body,
                    ..
                }) = &while_body_items[0]
                {
                    assert_eq!(*for_id, 1, "Inner for loop should have id 1");

                    if let checked::Statement::Compound(for_block) = &**for_body {
                        let for_body_items = &for_block.blocks;
                        assert_eq!(for_body_items.len(), 2);

                        // 检查 if { continue; } -> continue 的 target_id 应该是 1
                        if let checked::BlockItem::S(checked::Statement::If { then_stat, .. }) =
                            &for_body_items[0]
                        {
                            if let checked::Statement::Continue { target_id } = **then_stat {
                                assert_eq!(
                                    target_id, 1,
                                    "Continue should target inner for loop (id 1)"
                                );
                            } else {
                                panic!("Expected Continue statement");
                            }
                        } else {
                            panic!("Expected If statement");
                        }

                        // 检查 break; -> break 的 target_id 应该是 1
                        if let checked::BlockItem::S(checked::Statement::Break { target_id }) =
                            &for_body_items[1]
                        {
                            assert_eq!(
                                *target_id, 1,
                                "Inner break should target inner for loop (id 1)"
                            );
                        } else {
                            panic!("Expected Break statement");
                        }
                    } else {
                        panic!("For body should be a compound statement");
                    }
                } else {
                    panic!("Expected For loop");
                }

                // --- 检查外层的 break (target_id 应该是 0) ---
                if let checked::BlockItem::S(checked::Statement::Break { target_id }) =
                    &while_body_items[1]
                {
                    assert_eq!(
                        *target_id, 0,
                        "Outer break should target outer while loop (id 0)"
                    );
                } else {
                    panic!("Expected Break statement");
                }
            } else {
                panic!("While body should be a compound statement");
            }
        } else {
            panic!("Expected a While loop as the first statement");
        }

        println!("--- Loop Labeling Success Test Passed! ---");
    }

    #[test]
    fn test_break_outside_of_loop_fails() {
        // C 代码等价物: int main(void) { break; }
        let unchecked_ast = Program {
            function: Function {
                name: "main".to_string(),
                body: Block {
                    blocks: vec![BlockItem::S(Statement::Break)],
                },
            },
        };

        let mut id_gen = UniqueIdGenerator::new();
        let mut labeler = LoopLabeler::new(&mut id_gen);

        let result = labeler.label_program(unchecked_ast);
        assert!(
            result.is_err(),
            "Labeling should fail for a break outside a loop"
        );
        assert_eq!(result.unwrap_err(), "'break' statement not in a loop");
        println!("--- Break Outside Loop Failure Test Passed! ---");
    }
}
