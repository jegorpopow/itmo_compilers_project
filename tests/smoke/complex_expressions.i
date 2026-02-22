routine add_one(n : integer) : integer is
  return n + 1;
end;

routine main() is
  var a is 2;
  var b is 3;
  var c is 4;
  
  print 2 + 3 * 4;
  print (2 + 3) * 4;
  print 1 + 2 < 3 + 4;
  print 1 < 2 and 3 < 4;
  
  print (a + b) * (c - a);
  print a + b * c;
  
  print add_one(5) + add_one(3);
  print add_one(add_one(2));
  
  print (a < b) and (b < c);
  print (a > b) or (b < c);
  print not (a = b);
  
  print a + 1.5;
  print 2.0 * b;
end;

