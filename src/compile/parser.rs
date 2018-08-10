use super::ast;
use combine::char::{digit, newline, space, string, tab};
use combine::parser::combinator::try;
use combine::{error, many, many1, Parser, Stream};
/*
BNF
<program>  := {<stmt>}
<stmt>     := <skip_many> ( <infix> | <expr> ) <skip_many>
<expr>     := <num> <skip_many> { <op> <skip_many> <num> <skip_many> }
<infix>    := ('infixr' | 'infixl') <space>+ <num> <space>+ <op>
<op>       := '+' | '-' | '/' | '*'
<num>      := [0-9]+
<skip>     := '\n' | <space>
<skip_many>:= {<skip>}
<space>    := ' ' | '\t'

*/

pub fn parse(s: &str) -> Result<(ast::ProgramAST, &str), error::StringStreamError> {
    program_parser().parse(s)
}

//<program>
parser!{
   fn program_parser[I]()(I) ->ast::ProgramAST
    where [I: Stream<Item=char>]
    {
        many(stmt_parser()).map(|x|ast::ProgramAST{stmt_list:x})
    }
}

//<stmt>
parser!{
   fn stmt_parser[I]()(I) ->ast::StmtAST
    where [I: Stream<Item=char>]
    {
        skip_many_parser().
        with (
            infix_parser().map(|x|ast::StmtAST::InfixAST(x)).
            or(expr_parser().map(|x|ast::StmtAST::RawExpr(x)))
        ).
        skip(
            skip_many_parser()
        )
    }
}

//<expr>
parser!{
    fn expr_parser[I]()(I)->String
    where[I:Stream<Item=char>]
    {
        (
           num_parser().skip(skip_many_parser()),
           many((
                    op_parser().skip(skip_many_parser()),num_parser().skip(skip_many_parser())
               ))
        ).
       map(|(x,y):(String,Vec<(String,String)>)|{
           let mut s=x;
            y.into_iter().for_each(|(x,y)|{
                s+=&x;
                s+=&y;
            });
           s
       })
    }
}

//<infix>
parser!{
    fn infix_parser[I]()(I)->ast::InfixAST
    where[I:Stream<Item=char>]
    {
        (try(string("infixr")).
        or(string("infixl")).
        map(|s|{
            if s=="infixr"{
                ast::InfixType::Right
            }
            else{
                ast::InfixType::Left
            }
        }).
        skip(many1::<Vec<_>,_>(space_parser())),
        num_parser().
        skip(many1::<Vec<_>,_>(space_parser())),
        op_parser()
        ).
        map(|(ty,priority,op)|{
            ast::InfixAST{
                ty: ty,
                priority:priority.parse::<i8>().unwrap(),
                op:op
            }
        })
    }
}

//<op>
parser!{
    fn op_parser[I]()(I)->String
    where[I:Stream<Item=char>]
    {
        string("+").
        or(string("-")).
        or(string("/")).
        or(string("*")).map(|s|s.to_string())
    }
}

//<num>
parser!{
    fn num_parser[I]()(I)->String
    where[I:Stream<Item=char>]
    {
        many1(digit())
    }
}

//<skip>
parser!{
    fn skip_parser[I]()(I)->()
    where[I:Stream<Item=char>]
    {
        newline().map(|_|()).or(space_parser())
    }
}

//<skip_many>
parser!{
    fn skip_many_parser[I]()(I)->()
    where[I:Stream<Item=char>]
    {
        many::<Vec<_>,_>(skip_parser()).map(|_|())
    }
}

//<space>
parser!{
   fn space_parser[I]()(I) ->()
    where [I: Stream<Item=char>]
    {
        space().or(tab()).map(|_|())
    }
}
