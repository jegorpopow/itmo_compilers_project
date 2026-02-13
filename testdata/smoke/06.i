type point is record 
  var x is real;
  car y is real;
end;

type triangle is array [1 + 2] point;   

var EPS = 0.0000001;

routine approximately_eq(a : real, b : real, eps : real) => (a - b) * (a - b) < eps * eps;

routine squared_distance(from : point, to : point) : real is 
  return (from.x - to.x) * (from.x - to.x) + (from.y - to.y) * (from.y - to.y); 
end;

routine is_right(t : triangle) =>
     approximately_eq(squared_distance(t[1], t[2]) + squared_distance(t[2], t[3]), squared_distance(t[1], t[3]), EPS)
  or approximately_eq(squared_distance(t[1], t[3]) + squared_distance(t[1], t[2]), squared_distance(t[2], t[3]), EPS)
  or approximately_eq(squared_distance(t[3], t[1]) + squared_distance(t[3], t[2]), squared_distance(t[1], t[2]), EPS);
