routine f() => 1;
routine g() => 2;
routine h() => 3; 

routine main() is
  print 5 < 3 and 4 > 2;
  print true and false or 1 = 1;
  print g() + h() * f();
  print 17 % 5 % 2;
  print (g() + h()) * f();
  print 5 > 3 and 2 < 4 or not 1;
  print 100 / 2 / 5;
end;
