type linked_list is record 
  var data is integer;
  var tail is linked_list;
end;

var EMPTY : linked_list is null;

routine singleton(data: integer) => new linked_list where data is data; tail is EMPTY; end;

routine empty() => EMPTY;

routine is_empty(l : linked_list) => l = EMPTY;

routine length(l : linked_list ) : integer is 
  var result is 0;
  while not is_empty(l) loop
    result := result + 1;
    l := l.tail;
  end

  return result;
end

routine reverse_list(l : linked_list) : linked_list is
  var result is empty();

  while not is_empty(l) loop
    result := new linked_list where data is l.data; tail is result; end;
    l := l.tail;
  end

  return result;
end

routine main() is
  var list is singleton(1);
  print length(list);
end
