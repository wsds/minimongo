SELECT Books
WHERE book_type=$book_type AND[5] price>$max_price
ORDERBY price LIMIT 5 SKIP $skip
AS MathBooks

RETURN MathBooks
