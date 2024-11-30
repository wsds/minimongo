CREATE ONE Books CREATEONLY $books
UPDATE create_time = timestamp()
AS NewBook

SELECT Books
WHERE 15<price<20
ORDERBY num_liked LIMIT 552 SKIP 115
FIELD
    *,
    a=b+2c,
    like_books.select().orderby("num_liked").limit(30).skip(20),
    age.hide(),
AS MyBooks

SELECT ONE Users
WHERE username=abc
UPDATE
    like_books.push(NewBook),
    age=age+1,
    num_books=MyBooks.len(),
AS User1

GROUP Users BY friend
FIELD
    friend,
    num_friends=count(),
    friends=collect($record)
HAVING num_friends > 2 AND User1 IN friends
DELETE
AS Friends

RETURN User1       ,     Friends, Books
RETURN Books
