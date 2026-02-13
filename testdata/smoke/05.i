type point is record 
  var x is real;
  car y is real;
end;

routine point_of (x : real, y : real) : point is 
  var result : point;
  result.x := x;
  result.y := y;
  return result;
end;

routine squared_distance(from : point, to : point) : real is 
  return (from.x - to.x) * (from.x - to.x) + (from.y - to.y) * (from.y - to.y); 
end;

routine middle(a : point, b : point) => point_of((a.x + b.x) / 2, (a.y + b.y) / 2);
