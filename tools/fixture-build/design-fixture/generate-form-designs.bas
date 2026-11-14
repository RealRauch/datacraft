Option Compare Database
Option Explicit

' Generates the six-kind form-design fixture (step 78.3, item M88).
' Run from the generator script after LoadFromText(acModule) imported this
' module into a fresh, empty database. No error handler: a failure must
' surface as a COM error so the script fails fast.

Public Sub GenerateFormDesignFixtures()
    Dim db As DAO.Database
    Set db = CurrentDb
    db.Execute "CREATE TABLE Items (ID LONG NOT NULL, ItemName TEXT(50), StockLevel LONG, Active YESNO, CategoryId LONG)", dbFailOnError
    db.Execute "CREATE TABLE Categories (ID LONG NOT NULL, CategoryName TEXT(50))", dbFailOnError
    db.Execute "INSERT INTO Categories (ID, CategoryName) VALUES (1, 'Hardware')", dbFailOnError
    db.Execute "INSERT INTO Items (ID, ItemName, StockLevel, Active, CategoryId) VALUES (1, 'Bolt', 12, True, 1)", dbFailOnError

    BuildSubformSource
    BuildAllControlsForm
    BuildDefaultsForm
End Sub

' A minimal source form for the subform control.
Private Sub BuildSubformSource()
    Dim frm As Form
    Dim fname As String
    Set frm = CreateForm()
    fname = frm.Name
    frm.Caption = "Category Details"
    Set frm = Nothing
    DoCmd.Save acForm, fname
    DoCmd.Close acForm, fname, acSaveYes
    DoCmd.Rename "Category Details", acForm, fname
End Sub

' One form with every supported control kind and distinctive values.
Private Sub BuildAllControlsForm()
    Dim frm As Form
    Dim fname As String
    Dim c As Control
    Set frm = CreateForm()
    fname = frm.Name
    frm.RecordSource = "Items"
    frm.Caption = "All Controls"
    frm.Width = 8000
    frm.Section(acDetail).Height = 4000

    Set c = CreateControl(fname, acTextBox, acDetail, , "ItemName", 500, 400, 2000, 400)
    c.Name = "txtItemName"
    Set c = CreateControl(fname, acLabel, acDetail, , , 500, 1000, 2000, 300)
    c.Name = "lblItemName"
    c.Caption = "Item name"
    Set c = CreateControl(fname, acCheckBox, acDetail, , "Active", 500, 1600, 300, 300)
    c.Name = "chkActive"
    Set c = CreateControl(fname, acComboBox, acDetail, , "CategoryId", 500, 2200, 2000, 400)
    c.Name = "cboCategory"
    c.RowSourceType = "Table/Query"
    c.RowSource = "SELECT ID, CategoryName FROM Categories"
    c.ColumnCount = 2
    c.BoundColumn = 1
    c.ColumnWidths = "0cm;3cm"
    Set c = CreateControl(fname, acCommandButton, acDetail, , , 500, 2800, 2000, 500)
    c.Name = "cmdRun"
    c.Caption = "Run"
    Set c = CreateControl(fname, acSubform, acDetail, , , 3000, 400, 3000, 2000)
    c.Name = "subCategory"
    c.SourceObject = "Category Details"

    DoCmd.Save acForm, fname
    DoCmd.Close acForm, fname, acSaveYes
    DoCmd.Rename "All Controls", acForm, fname
End Sub

' One form whose controls are created with their default geometry, so the
' SaveAsText export pins the Access defaults the importer has to apply.
Private Sub BuildDefaultsForm()
    Dim frm As Form
    Dim fname As String
    Dim c As Control
    Set frm = CreateForm()
    fname = frm.Name
    frm.RecordSource = "Items"
    frm.Caption = "Defaults"

    Set c = CreateControl(fname, acLabel, acDetail)
    c.Name = "lblDefault"
    Set c = CreateControl(fname, acTextBox, acDetail, ColumnName:="ItemName")
    c.Name = "txtDefault"
    Set c = CreateControl(fname, acCheckBox, acDetail, ColumnName:="Active")
    c.Name = "chkDefault"
    Set c = CreateControl(fname, acComboBox, acDetail, ColumnName:="CategoryId")
    c.Name = "cboDefault"
    Set c = CreateControl(fname, acCommandButton, acDetail)
    c.Name = "cmdDefault"
    Set c = CreateControl(fname, acSubform, acDetail)
    c.Name = "subDefault"
    c.SourceObject = "Category Details"

    DoCmd.Save acForm, fname
    DoCmd.Close acForm, fname, acSaveYes
    DoCmd.Rename "Defaults", acForm, fname
End Sub
