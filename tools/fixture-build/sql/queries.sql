Items In Stock|SELECT ItemId, ItemName, StockLevel FROM [Warehouse Items] WHERE StockLevel > 0 ORDER BY ItemName
Stock By Category|SELECT c.CategoryName, COUNT(*) AS ItemCount FROM Categories AS c INNER JOIN [Warehouse Items] AS i ON c.CategoryId = i.CategoryId GROUP BY c.CategoryName HAVING COUNT(*) > 0
Items With Suppliers|SELECT i.ItemName, s.SupplierName FROM [Warehouse Items] AS i INNER JOIN Suppliers AS s ON i.SupplierId = s.SupplierId ORDER BY i.ItemName
Items Without Movements|SELECT i.ItemName, m.MovedOn FROM [Warehouse Items] AS i LEFT JOIN [Stock Movements] AS m ON i.ItemId = m.ItemId ORDER BY i.ItemName
Suppliers Without Items|SELECT s.SupplierName, i.ItemName FROM Suppliers AS s RIGHT JOIN [Warehouse Items] AS i ON s.SupplierId = i.SupplierId ORDER BY s.SupplierName
Movements In Range|PARAMETERS [Start Date] DateTime, [End Date] DateTime; SELECT m.MovedOn, m.Quantity, i.ItemName FROM [Stock Movements] AS m INNER JOIN [Warehouse Items] AS i ON m.ItemId = i.ItemId WHERE m.MovedOn BETWEEN [Start Date] AND [End Date]
Distinct Categories|SELECT DISTINCT CategoryName FROM Categories ORDER BY CategoryName
Top Items In Stock|SELECT TOP 5 ItemName, StockLevel FROM [Warehouse Items] ORDER BY StockLevel DESC
Tagged Items|SELECT ItemName, Tags.Value FROM [Warehouse Items]
