type linked_list is record 
  var data : int;
  var tail : linked_list
end;

routine linked_list_of(data: int, tail : linked_list) is 
  var result : linked_list;
  result.data = data;
  result.tail = tail; 
  return result;
end;

var EMPTY : linked_list;

routine singleton(data: int) => linked_list_of(data, EMPTY);

routine empty() => EMPTY;

routine is_empty(l : linked_list) => l == EMPTY;

routine length(l : linked_list ) : int is 
  var result is 0;
  while is_empty(l) != true loop:
    result := result + 1;
    l := l.tail;
  end;

  return result;
end;

routine reverse(l : linked_list) : linked_list is
  var result is empty();

  while is_empty(l) != true loop:
    result = linked_list_of(l.data, result);
    l = l.tail;
  end;

  return result;
end;


