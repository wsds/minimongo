SELECT Books
WHERE book_type="Math" OR[5] book_type="Physics"
ORDERBY price LIMIT 10 SKIP 0
FIELD    *,    a=b+2c,
AS MathBooks

SELECT Books
WHERE book_type="Physics"
ORDERBY price LIMIT 10 SKIP 0 DESC
FIELD    *,    a=b+2c,
AS PhysicsBooks

SELECT Books
AS AllBooks

SELECT Books
WHERE book_type="Physics"
ORDERBY price LIMIT 10 SKIP 3 ASC
FIELD name,book_uid,price
AS CheapBooks

SELECT Books
ORDERBY price LIMIT 5 SKIP 3 ASC
FIELD name,book_uid,price
AS CheapBooks

SELECT ONE Books
WHERE NOT book_type="Physics"
FIELD book_uid,book_type,pa
AS SeeBooks

RETURN MathBooks, PhysicsBooks, SeeBooks
